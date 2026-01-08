use crate::state::State;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::Utc;
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// ==================== LIMITS ====================
pub const MAX_VARIABLES: usize = 10;
pub const MAX_MAPPINGS: usize = 5;
pub const MAX_FUNCTIONS: usize = 10;
pub const MAX_OPS_PER_FUNCTION: usize = 20;
pub const MAX_STRING_LENGTH: usize = 256;
pub const MAX_NAME_LENGTH: usize = 32;

// ==================== TYPES ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VarType {
    Uint64,
    String,
    Bool,
    Address,
}

impl VarType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // All uint variants → Uint64 (we store as u64 internally)
            "uint64" | "uint" | "number" | "uint256" | "uint128" | "uint32" | "uint16" | "uint8" => Some(VarType::Uint64),
            // All int variants → Uint64 (simplified, no negative support yet)
            "int256" | "int128" | "int64" | "int32" | "int" => Some(VarType::Uint64),
            "string" | "str" => Some(VarType::String),
            "bool" | "boolean" => Some(VarType::Bool),
            "address" | "addr" => Some(VarType::Address),
            _ => None,
        }
    }
}

// ==================== CONTRACT SCHEMA ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDef {
    pub name: String,
    pub var_type: VarType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingDef {
    pub name: String,
    pub key_type: VarType,
    pub value_type: VarType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnArg {
    pub name: String,
    pub arg_type: VarType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FnModifier {
    View,
    Write,
    Payable,
    OnlyOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnDef {
    pub name: String,
    #[serde(default)]
    pub modifiers: Vec<FnModifier>,
    #[serde(default)]
    pub args: Vec<FnArg>,
    #[serde(default)]
    pub body: Vec<Operation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns: Option<VarType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoshContract {
    // Identity
    pub address: String,
    pub name: String,
    pub creator: String,
    pub owner: String,
    pub created_at: i64,
    
    // Token (optional)
    pub token: Option<String>,
    
    // Schema
    pub variables: Vec<VarDef>,
    pub mappings: Vec<MappingDef>,
    pub functions: Vec<FnDef>,
}

// ==================== EXECUTION CONTEXT ====================

#[derive(Debug, Clone)]
pub struct ExecContext {
    pub caller: String,
    pub amount: u64,          // For payable
    pub block_height: u64,
    pub block_timestamp: u64,
    pub args: HashMap<String, String>,  // Function arguments
    pub locals: HashMap<String, String>, // Local variables during execution
}

// ==================== CALL RESULT ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub gas_used: u64,
}

impl CallResult {
    pub fn ok(data: serde_json::Value, gas: u64) -> Self {
        CallResult { success: true, data: Some(data), error: None, gas_used: gas }
    }
    pub fn err(msg: &str, gas: u64) -> Self {
        CallResult { success: false, data: None, error: Some(msg.to_string()), gas_used: gas }
    }
}

// ==================== MVM ENGINE ====================

pub struct MVM;

impl MVM {
    pub fn new() -> Self { MVM }

    /// Deploy a new Mosh contract
    pub fn deploy(
        &self,
        state: &mut State,
        creator: &str,
        name: &str,
        token: Option<String>,
        variables: Vec<VarDef>,
        mappings: Vec<MappingDef>,
        functions: Vec<FnDef>,
    ) -> Result<String, BoxError> {
        
        // Validate name
        if name.is_empty() || name.len() > MAX_NAME_LENGTH {
            return Err(format!("Name: 1-{} chars", MAX_NAME_LENGTH).into());
        }
        
        // Validate counts
        if variables.len() > MAX_VARIABLES {
            return Err(format!("Max {} variables", MAX_VARIABLES).into());
        }
        if mappings.len() > MAX_MAPPINGS {
            return Err(format!("Max {} mappings", MAX_MAPPINGS).into());
        }
        if functions.len() > MAX_FUNCTIONS {
            return Err(format!("Max {} functions", MAX_FUNCTIONS).into());
        }
        
        // Check duplicates
        let mut names = std::collections::HashSet::new();
        let reserved = ["owner", "creator", "token", "address", "balance"];
        
        for v in &variables {
            if reserved.contains(&v.name.as_str()) {
                return Err(format!("Reserved: {}", v.name).into());
            }
            if !names.insert(v.name.clone()) {
                return Err(format!("Duplicate: {}", v.name).into());
            }
        }
        for m in &mappings {
            if !names.insert(m.name.clone()) {
                return Err(format!("Duplicate: {}", m.name).into());
            }
        }
        for f in &functions {
            if f.body.len() > MAX_OPS_PER_FUNCTION {
                return Err(format!("Function {} has too many ops (max {})", f.name, MAX_OPS_PER_FUNCTION).into());
            }
        }
        
        // Validate token
        if let Some(ref t) = token {
            if state.get_token(t)?.is_none() {
                return Err(format!("Token not found: {}", t).into());
            }
        }
        
        // Generate address
        let mut hasher = Sha256::new();
        hasher.update(creator.as_bytes());
        hasher.update(name.as_bytes());
        hasher.update(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        let hash = hasher.finalize();
        let address = format!("mvm1contract{}", hex::encode(&hash[..10]));
        
        let contract = MoshContract {
            address: address.clone(),
            name: name.to_string(),
            creator: creator.to_string(),
            owner: creator.to_string(),
            created_at: Utc::now().timestamp(),
            token,
            variables: variables.clone(),
            mappings,
            functions,
        };
        
        state.save_mosh_contract(&contract)?;
        
        // Initialize variables
        for v in &variables {
            let val = v.default.clone().unwrap_or_else(|| match v.var_type {
                VarType::Uint64 => "0".to_string(),
                VarType::String => "".to_string(),
                VarType::Bool => "false".to_string(),
                VarType::Address => "".to_string(),
            });
            state.set_mosh_var(&address, &v.name, &val)?;
        }
        
        Ok(address)
    }

    /// Call a contract function
    pub fn call(
        &self,
        state: &mut State,
        caller: &str,
        contract_addr: &str,
        fn_name: &str,
        args: Vec<String>,
        amount: u64, // For payable
    ) -> Result<CallResult, BoxError> {
        
        let contract = state.get_mosh_contract(contract_addr)?
            .ok_or_else(|| BoxError::from("Contract not found"))?;
        
        let mut gas: u64 = 5000;
        let now = Utc::now().timestamp() as u64;
        
        // ========== AUTO GETTERS ==========
        // get_<var> - auto generated for all variables
        if fn_name.starts_with("get_") {
            let var_name = &fn_name[4..];
            gas += 1000;
            
            // Reserved getters
            match var_name {
                "owner" => return Ok(CallResult::ok(serde_json::json!(contract.owner), gas)),
                "creator" => return Ok(CallResult::ok(serde_json::json!(contract.creator), gas)),
                "token" => return Ok(CallResult::ok(serde_json::json!(contract.token), gas)),
                "address" => return Ok(CallResult::ok(serde_json::json!(contract.address), gas)),
                _ => {}
            }
            
            // User variable
            if let Some(v) = contract.variables.iter().find(|x| x.name == var_name) {
                let val = state.get_mosh_var(contract_addr, var_name)?.unwrap_or_default();
                return Ok(CallResult::ok(self.typed_value(&val, &v.var_type), gas));
            }
            
            // Mapping: get_mapname(key)
            if let Some(m) = contract.mappings.iter().find(|x| x.name == var_name) {
                if args.is_empty() {
                    return Ok(CallResult::err("Missing key", gas));
                }
                let val = state.get_mosh_map(contract_addr, var_name, &args[0])?.unwrap_or_default();
                return Ok(CallResult::ok(serde_json::json!({
                    "key": &args[0],
                    "value": self.typed_value(&val, &m.value_type)
                }), gas));
            }
            
            return Ok(CallResult::err(&format!("Unknown: {}", var_name), gas));
        }
        
        // ========== AUTO SETTERS (Owner only) ==========
        if fn_name.starts_with("set_") {
            let var_name = &fn_name[4..];
            gas += 5000;
            
            // Owner check
            if caller != contract.owner {
                return Ok(CallResult::err("Only owner", gas));
            }
            
            // Transfer ownership
            if var_name == "owner" {
                if args.is_empty() {
                    return Ok(CallResult::err("Missing address", gas));
                }
                let mut updated = contract.clone();
                updated.owner = args[0].clone();
                state.save_mosh_contract(&updated)?;
                return Ok(CallResult::ok(serde_json::json!({"new_owner": &args[0]}), gas));
            }
            
            // User variable
            if let Some(v) = contract.variables.iter().find(|x| x.name == var_name) {
                if args.is_empty() {
                    return Ok(CallResult::err("Missing value", gas));
                }
                state.set_mosh_var(contract_addr, var_name, &args[0])?;
                return Ok(CallResult::ok(self.typed_value(&args[0], &v.var_type), gas));
            }
            
            // Mapping: set_mapname(key, value)
            if contract.mappings.iter().any(|x| x.name == var_name) {
                if args.len() < 2 {
                    return Ok(CallResult::err("Need: key, value", gas));
                }
                state.set_mosh_map(contract_addr, var_name, &args[0], &args[1])?;
                return Ok(CallResult::ok(serde_json::json!({"key": &args[0], "value": &args[1]}), gas));
            }
            
            return Ok(CallResult::err(&format!("Unknown: {}", var_name), gas));
        }
        
        // ========== USER DEFINED FUNCTIONS ==========
        let func = contract.functions.iter().find(|f| f.name == fn_name);
        if func.is_none() {
            return Ok(CallResult::err(&format!("Function not found: {}", fn_name), gas));
        }
        let func = func.unwrap();
        
        gas += 10000;
        
        // Check modifiers
        if func.modifiers.contains(&FnModifier::OnlyOwner) && caller != contract.owner {
            return Ok(CallResult::err("Only owner", gas));
        }
        if func.modifiers.contains(&FnModifier::Payable) {
            if contract.token.is_none() {
                return Ok(CallResult::err("No token linked", gas));
            }
        }
        if !func.modifiers.contains(&FnModifier::Payable) && amount > 0 {
            return Ok(CallResult::err("Function not payable", gas));
        }
        
        // Build context
        let mut ctx = ExecContext {
            caller: caller.to_string(),
            amount,
            block_height: state.get_height().unwrap_or(0),
            block_timestamp: now,
            args: HashMap::new(),
            locals: HashMap::new(),
        };
        
        // Map args
        for (i, arg_def) in func.args.iter().enumerate() {
            let val = args.get(i).cloned().unwrap_or_default();
            ctx.args.insert(arg_def.name.clone(), val);
        }
        
        // Handle payable - transfer tokens from caller to contract
        if func.modifiers.contains(&FnModifier::Payable) && amount > 0 {
            let token_addr = contract.token.as_ref().unwrap();
            let caller_bal = state.get_token_balance(token_addr, caller)?;
            if caller_bal < amount {
                return Ok(CallResult::err(&format!("Insufficient: {} < {}", caller_bal, amount), gas));
            }
            state.set_token_balance(token_addr, caller, caller_bal - amount)?;
            let contract_bal = state.get_token_balance(token_addr, contract_addr)?;
            state.set_token_balance(token_addr, contract_addr, contract_bal + amount)?;
        }
        
        // Execute operations
        let mut return_value: Option<serde_json::Value> = None;
        
        for op in &func.body {
            gas += 1000;
            
            match op.op.as_str() {
                // SET variable
                "set" => {
                    let var = op.var.as_deref().unwrap_or("");
                    let value = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    state.set_mosh_var(contract_addr, var, &value)?;
                }
                
                // ADD to variable
                "add" => {
                    let var = op.var.as_deref().unwrap_or("");
                    let add_val = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    let current = state.get_mosh_var(contract_addr, var)?.unwrap_or("0".to_string());
                    let new_val = current.parse::<u64>().unwrap_or(0) + add_val.parse::<u64>().unwrap_or(0);
                    state.set_mosh_var(contract_addr, var, &new_val.to_string())?;
                }
                
                // SUB from variable
                "sub" => {
                    let var = op.var.as_deref().unwrap_or("");
                    let sub_val = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    let current = state.get_mosh_var(contract_addr, var)?.unwrap_or("0".to_string());
                    let new_val = current.parse::<u64>().unwrap_or(0).saturating_sub(sub_val.parse::<u64>().unwrap_or(0));
                    state.set_mosh_var(contract_addr, var, &new_val.to_string())?;
                }
                
                // MAP_SET
                "map_set" => {
                    let map = op.map.as_deref().unwrap_or("");
                    let key = self.resolve_value(state, &contract, &ctx, op.key.as_ref())?;
                    let value = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    state.set_mosh_map(contract_addr, map, &key, &value)?;
                }
                
                // MAP_ADD
                "map_add" => {
                    let map = op.map.as_deref().unwrap_or("");
                    let key = self.resolve_value(state, &contract, &ctx, op.key.as_ref())?;
                    let add_val = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    let current = state.get_mosh_map(contract_addr, map, &key)?.unwrap_or("0".to_string());
                    let new_val = current.parse::<u64>().unwrap_or(0) + add_val.parse::<u64>().unwrap_or(0);
                    state.set_mosh_map(contract_addr, map, &key, &new_val.to_string())?;
                }
                
                // MAP_SUB
                "map_sub" => {
                    let map = op.map.as_deref().unwrap_or("");
                    let key = self.resolve_value(state, &contract, &ctx, op.key.as_ref())?;
                    let sub_val = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    let current = state.get_mosh_map(contract_addr, map, &key)?.unwrap_or("0".to_string());
                    let new_val = current.parse::<u64>().unwrap_or(0).saturating_sub(sub_val.parse::<u64>().unwrap_or(0));
                    state.set_mosh_map(contract_addr, map, &key, &new_val.to_string())?;
                }
                
                // REQUIRE - check condition
                "require" => {
                    let left = self.resolve_value(state, &contract, &ctx, op.left.as_ref())?;
                    let cmp = op.cmp.as_deref().unwrap_or(">");
                    let right = self.resolve_value(state, &contract, &ctx, op.right.as_ref())?;
                    let msg = op.msg.as_deref().unwrap_or("Require failed");
                    
                    let left_num = left.parse::<u64>().unwrap_or(0);
                    let right_num = right.parse::<u64>().unwrap_or(0);
                    
                    let pass = match cmp {
                        ">" => left_num > right_num,
                        ">=" => left_num >= right_num,
                        "<" => left_num < right_num,
                        "<=" => left_num <= right_num,
                        "==" | "=" => left == right,
                        "!=" => left != right,
                        _ => false,
                    };
                    
                    if !pass {
                        return Ok(CallResult::err(msg, gas));
                    }
                }
                
                // TRANSFER tokens from contract to address
                "transfer" => {
                    let token_addr = match &contract.token {
                        Some(t) => t.clone(),
                        None => return Ok(CallResult::err("No token", gas)),
                    };
                    
                    let to = self.resolve_value(state, &contract, &ctx, op.to.as_ref())?;
                    let amt = self.resolve_value(state, &contract, &ctx, op.amount.as_ref())?;
                    let amt_num = amt.parse::<u64>().unwrap_or(0);
                    
                    let contract_bal = state.get_token_balance(&token_addr, contract_addr)?;
                    if contract_bal < amt_num {
                        return Ok(CallResult::err("Contract balance low", gas));
                    }
                    
                    state.set_token_balance(&token_addr, contract_addr, contract_bal - amt_num)?;
                    let to_bal = state.get_token_balance(&token_addr, &to)?;
                    state.set_token_balance(&token_addr, &to, to_bal + amt_num)?;
                }
                
                // RETURN value
                "return" => {
                    let val = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    return_value = Some(serde_json::json!(val));
                }
                
                // LET - local variable
                "let" => {
                    let var = op.var.as_deref().unwrap_or("");
                    let value = self.resolve_value(state, &contract, &ctx, op.value.as_ref())?;
                    ctx.locals.insert(var.to_string(), value);
                }
                
                _ => {
                    return Ok(CallResult::err(&format!("Unknown op: {}", op.op), gas));
                }
            }
        }
        
        Ok(CallResult::ok(return_value.unwrap_or(serde_json::json!({"success": true})), gas))
    }
    
    /// Resolve a value - can be literal, variable, mapping, or special
    fn resolve_value(
        &self,
        state: &State,
        contract: &MoshContract,
        ctx: &ExecContext,
        val: Option<&serde_json::Value>,
    ) -> Result<String, BoxError> {
        let val = match val {
            Some(v) => v,
            None => return Ok("0".to_string()),
        };
        
        // String literal
        if let Some(s) = val.as_str() {
            // Special values
            match s {
                "msg.sender" => return Ok(ctx.caller.clone()),
                "msg.amount" => return Ok(ctx.amount.to_string()),
                "block.height" => return Ok(ctx.block_height.to_string()),
                "block.timestamp" => return Ok(ctx.block_timestamp.to_string()),
                "contract.owner" => return Ok(contract.owner.clone()),
                "contract.address" => return Ok(contract.address.clone()),
                _ => {}
            }
            
            // Check if it's an argument
            if let Some(arg_val) = ctx.args.get(s) {
                return Ok(arg_val.clone());
            }
            
            // Check if it's a local variable
            if let Some(local_val) = ctx.locals.get(s) {
                return Ok(local_val.clone());
            }
            
            // Check if it's a contract variable
            if contract.variables.iter().any(|v| v.name == s) {
                return Ok(state.get_mosh_var(&contract.address, s)?.unwrap_or_default());
            }
            
            // Check if it's a mapping access: mapname[key]
            if s.contains('[') && s.ends_with(']') {
                let parts: Vec<&str> = s.trim_end_matches(']').split('[').collect();
                if parts.len() == 2 {
                    let map_name = parts[0];
                    let key_expr = parts[1];
                    let key = self.resolve_value(state, contract, ctx, Some(&serde_json::json!(key_expr)))?;
                    return Ok(state.get_mosh_map(&contract.address, map_name, &key)?.unwrap_or_default());
                }
            }
            
            // Return as literal
            return Ok(s.to_string());
        }
        
        // Number literal
        if let Some(n) = val.as_u64() {
            return Ok(n.to_string());
        }
        if let Some(n) = val.as_i64() {
            return Ok(n.to_string());
        }
        
        // Boolean
        if let Some(b) = val.as_bool() {
            return Ok(b.to_string());
        }
        
        Ok(val.to_string())
    }
    
    fn typed_value(&self, val: &str, var_type: &VarType) -> serde_json::Value {
        match var_type {
            VarType::Uint64 => serde_json::json!(val.parse::<u64>().unwrap_or(0)),
            VarType::Bool => serde_json::json!(val == "true"),
            VarType::String | VarType::Address => serde_json::json!(val),
        }
    }

    /// Legacy compatibility
    pub fn execute_call(
        &mut self,
        state: &mut State,
        contract: &str,
        method: &str,
        args: &[String],
    ) -> Result<Option<serde_json::Value>, BoxError> {
        if contract.starts_with("mvm1contract") {
            let result = self.call(state, "", contract, method, args.to_vec(), 0)?;
            if result.success { Ok(result.data) } else { Err(result.error.unwrap_or("Error".into()).into()) }
        } else {
            if method == "set" && !args.is_empty() {
                state.set_mosh_var(contract, "value", &args[0])?;
                Ok(None)
            } else if method == "get" {
                Ok(state.get_mosh_var(contract, "value")?.map(|v| serde_json::json!(v)))
            } else {
                Err(format!("Unknown: {}", method).into())
            }
        }
    }
}

impl Default for MVM {
    fn default() -> Self { Self::new() }
}