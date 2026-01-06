use crate::state::State;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Nop = 0x00,
    Push = 0x01,
    Pop = 0x02,
    Dup = 0x03,
    Swap = 0x04,
    Store = 0x10,
    Load = 0x11,
    SStore = 0x12,
    SLoad = 0x13,
    Return = 0x20,
    Revert = 0x21,
    Call = 0x30,
    Add = 0x40,
    Sub = 0x41,
    Mul = 0x42,
    Div = 0x43,
    Eq = 0x50,
    Gt = 0x51,
    Lt = 0x52,
    Stop = 0xFF,
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => OpCode::Nop,
            0x01 => OpCode::Push,
            0x02 => OpCode::Pop,
            0x03 => OpCode::Dup,
            0x04 => OpCode::Swap,
            0x10 => OpCode::Store,
            0x11 => OpCode::Load,
            0x12 => OpCode::SStore,
            0x13 => OpCode::SLoad,
            0x20 => OpCode::Return,
            0x21 => OpCode::Revert,
            0x30 => OpCode::Call,
            0x40 => OpCode::Add,
            0x41 => OpCode::Sub,
            0x42 => OpCode::Mul,
            0x43 => OpCode::Div,
            0x50 => OpCode::Eq,
            0x51 => OpCode::Gt,
            0x52 => OpCode::Lt,
            0xFF => OpCode::Stop,
            _ => OpCode::Nop,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MVMValue {
    Uint(u64),
    String(String),
    Bool(bool),
    Bytes(Vec<u8>),
}

impl MVMValue {
    pub fn to_u64(&self) -> u64 {
        match self {
            MVMValue::Uint(v) => *v,
            MVMValue::Bool(b) => if *b { 1 } else { 0 },
            _ => 0,
        }
    }

    pub fn to_string_value(&self) -> String {
        match self {
            MVMValue::String(s) => s.clone(),
            MVMValue::Uint(v) => v.to_string(),
            MVMValue::Bool(b) => b.to_string(),
            MVMValue::Bytes(b) => hex::encode(b),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub address: String,
    pub creator: String,
    pub bytecode: Vec<u8>,
    pub name: String,
    pub abi: Vec<ContractFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractFunction {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub is_view: bool,
}

pub struct MVM {
    stack: Vec<MVMValue>,
    memory: HashMap<String, MVMValue>,
    gas_used: u64,
    gas_limit: u64,
}

impl MVM {
    pub fn new() -> Self {
        MVM {
            stack: Vec::new(),
            memory: HashMap::new(),
            gas_used: 0,
            gas_limit: 1_000_000,
        }
    }

    pub fn execute_call(
        &mut self,
        state: &mut State,
        contract: &str,
        method: &str,
        args: &[String],
    ) -> Result<Option<MVMValue>, BoxError> {
        if method == "set" && !args.is_empty() {
            let value = args[0].parse::<u64>().unwrap_or(0);
            state.set_contract_storage(contract, "value", &serde_json::to_string(&value)?)?;
            Ok(None)
        } else if method == "get" {
            let storage = state.get_contract_storage(contract, "value")?;
            if let Some(val) = storage {
                let value: u64 = serde_json::from_str(&val)?;
                Ok(Some(MVMValue::Uint(value)))
            } else {
                Ok(Some(MVMValue::Uint(0)))
            }
        } else {
            Err(format!("Unknown method: {}", method).into())
        }
    }

    pub fn execute_bytecode(
        &mut self,
        bytecode: &[u8],
        state: &mut State,
        contract: &str,
    ) -> Result<Option<MVMValue>, BoxError> {
        let mut pc = 0;
        
        while pc < bytecode.len() {
            let opcode = OpCode::from(bytecode[pc]);
            pc += 1;

            self.gas_used += 1;
            if self.gas_used > self.gas_limit {
                return Err("Out of gas".into());
            }

            match opcode {
                OpCode::Stop => break,
                OpCode::Push => {
                    if pc >= bytecode.len() {
                        return Err("Invalid bytecode: missing push length".into());
                    }
                    let len = bytecode[pc] as usize;
                    pc += 1;
                    if pc + len > bytecode.len() {
                        return Err("Invalid bytecode: missing push data".into());
                    }
                    let data = &bytecode[pc..pc + len];
                    pc += len;
                    
                    if len <= 8 {
                        let mut bytes = [0u8; 8];
                        bytes[..len].copy_from_slice(data);
                        self.stack.push(MVMValue::Uint(u64::from_le_bytes(bytes)));
                    } else {
                        self.stack.push(MVMValue::Bytes(data.to_vec()));
                    }
                }
                OpCode::Pop => {
                    self.stack.pop().ok_or("Stack underflow")?;
                }
                OpCode::Dup => {
                    let top = self.stack.last().ok_or("Stack underflow")?.clone();
                    self.stack.push(top);
                }
                OpCode::Add => {
                    let b = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    let a = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    self.stack.push(MVMValue::Uint(a + b));
                }
                OpCode::Sub => {
                    let b = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    let a = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    self.stack.push(MVMValue::Uint(a.saturating_sub(b)));
                }
                OpCode::Mul => {
                    let b = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    let a = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    self.stack.push(MVMValue::Uint(a * b));
                }
                OpCode::Div => {
                    let b = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    let a = self.stack.pop().ok_or("Stack underflow")?.to_u64();
                    if b == 0 {
                        return Err("Division by zero".into());
                    }
                    self.stack.push(MVMValue::Uint(a / b));
                }
                OpCode::SStore => {
                    let value = self.stack.pop().ok_or("Stack underflow")?;
                    let key = self.stack.pop().ok_or("Stack underflow")?.to_string_value();
                    state.set_contract_storage(contract, &key, &value.to_string_value())?;
                }
                OpCode::SLoad => {
                    let key = self.stack.pop().ok_or("Stack underflow")?.to_string_value();
                    if let Some(value) = state.get_contract_storage(contract, &key)? {
                        self.stack.push(MVMValue::String(value));
                    } else {
                        self.stack.push(MVMValue::Uint(0));
                    }
                }
                OpCode::Return => {
                    return Ok(self.stack.pop());
                }
                _ => {}
            }
        }

        Ok(self.stack.pop())
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.memory.clear();
        self.gas_used = 0;
    }
}

impl Default for MVM {
    fn default() -> Self {
        Self::new()
    }
}
