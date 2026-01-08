use crate::config::Config;
use crate::state::State;
use crate::address::Address;
use crate::mvm::MVM;

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Transaction error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxError {
    InvalidSignature { message: String },
    InvalidNonce { expected: u64, got: u64 },
    InsufficientBalance { required: u64, available: u64 },
    InvalidAddress { address: String },
    InvalidRecipient { message: String },
    TokenNotFound { contract: String },
    InsufficientTokenBalance { required: u64, available: u64 },
    ContractError { message: String },
    InvalidTxType { tx_type: String },
    GasExceeded { limit: u64, used: u64 },
    InternalError { message: String },
}

impl std::fmt::Display for TxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxError::InvalidSignature { message } => write!(f, "Invalid signature: {}", message),
            TxError::InvalidNonce { expected, got } => write!(f, "Invalid nonce: expected {}, got {}", expected, got),
            TxError::InsufficientBalance { required, available } => write!(f, "Insufficient balance: need {}, have {}", required, available),
            TxError::InvalidAddress { address } => write!(f, "Invalid address: {}", address),
            TxError::InvalidRecipient { message } => write!(f, "Invalid recipient: {}", message),
            TxError::TokenNotFound { contract } => write!(f, "Token not found: {}", contract),
            TxError::InsufficientTokenBalance { required, available } => write!(f, "Insufficient token balance: need {}, have {}", required, available),
            TxError::ContractError { message } => write!(f, "Contract error: {}", message),
            TxError::InvalidTxType { tx_type } => write!(f, "Invalid transaction type: {}", tx_type),
            TxError::GasExceeded { limit, used } => write!(f, "Gas exceeded: limit {}, used {}", limit, used),
            TxError::InternalError { message } => write!(f, "Internal error: {}", message),
        }
    }
}

impl std::error::Error for TxError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: i64,
    pub validator: String,
    pub transactions: Vec<Transaction>,
    pub tx_count: usize,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub rewards: BlockRewards,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRewards {
    pub validator_reward: u64,
    pub service_rewards: Vec<ServiceReward>,
    pub total_minted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceReward {
    pub rank: u8,
    pub node_id: String,
    pub address: String,
    pub browsers: u32,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub tx_type: TxType,
    pub from: String,
    pub to: Option<String>,
    pub value: u64,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub nonce: u64,
    pub data: Option<TxData>,
    pub timestamp: i64,
    pub signature: String,
    pub public_key: String,
    pub status: TxStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TxType {
    Transfer,
    Deploy,
    Call,
    CreateToken,
    TransferToken,
    DeployContract,
    CallContract,
}

impl TxType {
    pub fn as_str(&self) -> &str {
        match self {
            TxType::Transfer => "transfer",
            TxType::Deploy => "deploy",
            TxType::Call => "call",
            TxType::CreateToken => "create_token",
            TxType::TransferToken => "transfer_token",
            TxType::DeployContract => "deploy_contract",
            TxType::CallContract => "call_contract",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxData {
    Deploy { code: Vec<u8>, name: String },
    Call { contract: String, method: String, args: Vec<String> },
    CreateToken { name: String, symbol: String, total_supply: u64 },
    TransferToken { contract: String, to: String, amount: u64 },
    // Mosh Contract Deployment
    DeployContract { 
        name: String, 
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(default)]
        variables: Vec<crate::mvm::VarDef>,
        #[serde(default)]
        mappings: Vec<crate::mvm::MappingDef>,
        #[serde(default)]
        functions: Vec<crate::mvm::FnDef>,
    },
    // Mosh Contract Call
    CallContract { 
        contract: String, 
        method: String, 
        #[serde(default)]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        amount: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TxStatus {
    Pending,
    Success,
    Failed,
}

impl Transaction {
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", self.tx_type));
        hasher.update(&self.from);
        hasher.update(self.to.as_deref().unwrap_or(""));
        hasher.update(self.value.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get the message that needs to be signed
    pub fn get_sign_message(&self) -> Vec<u8> {
        let data_str = self.data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
        crate::address::hash_tx_data(
            self.tx_type.as_str(),
            &self.from,
            self.to.as_deref(),
            self.value,
            self.nonce,
            data_str.as_deref(),
        )
    }

    /// Verify the transaction signature
    pub fn verify_signature(&self) -> Result<bool, BoxError> {
        let message = self.get_sign_message();
        crate::address::verify_tx_signature(
            &self.from,
            &message,
            &self.signature,
            &self.public_key,
        )
    }
}

impl Block {
    pub fn genesis(master_address: &str, master_balance: u64) -> Self {
        let timestamp = Utc::now().timestamp();
        let mut block = Block {
            height: 0,
            hash: String::new(),
            prev_hash: "0".repeat(64),
            timestamp,
            validator: master_address.to_string(),
            transactions: vec![],
            tx_count: 0,
            gas_used: 0,
            gas_limit: 1_000_000,
            rewards: BlockRewards {
                validator_reward: master_balance,
                service_rewards: vec![],
                total_minted: master_balance,
            },
            signature: String::new(),
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn new(
        height: u64,
        prev_hash: &str,
        validator: &str,
        transactions: Vec<Transaction>,
        rewards: BlockRewards,
        gas_limit: u64,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let tx_count = transactions.len();
        let gas_used: u64 = transactions.iter().map(|tx| tx.gas_used).sum();
        
        let mut block = Block {
            height,
            hash: String::new(),
            prev_hash: prev_hash.to_string(),
            timestamp,
            validator: validator.to_string(),
            transactions,
            tx_count,
            gas_used,
            gas_limit,
            rewards,
            signature: String::new(),
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(&self.prev_hash);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(&self.validator);
        for tx in &self.transactions {
            hasher.update(&tx.hash);
        }
        hex::encode(hasher.finalize())
    }

    pub fn is_valid(&self) -> bool {
        self.hash == self.calculate_hash()
    }
}

/// Transaction pool with nonce ordering and deduplication
#[derive(Debug, Default)]
pub struct Mempool {
    /// All pending transactions by hash (for deduplication)
    pub by_hash: std::collections::HashMap<String, Transaction>,
    /// Transactions grouped by sender, sorted by nonce
    pub by_sender: std::collections::HashMap<String, std::collections::BTreeMap<u64, String>>,
    /// Total count
    pub count: usize,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool {
            by_hash: std::collections::HashMap::new(),
            by_sender: std::collections::HashMap::new(),
            count: 0,
        }
    }
    
    /// Add transaction to mempool
    /// Returns Ok(true) if added, Ok(false) if duplicate hash, Err if same sender+nonce exists
    pub fn add(&mut self, tx: Transaction) -> Result<bool, String> {
        let hash = tx.hash.clone();
        let sender = tx.from.clone();
        let nonce = tx.nonce;
        
        // Check duplicate hash
        if self.by_hash.contains_key(&hash) {
            return Ok(false);
        }
        
        // Check if same sender+nonce already exists - REJECT (not replace)
        if let Some(sender_txs) = self.by_sender.get(&sender) {
            if sender_txs.contains_key(&nonce) {
                return Err(format!("Transaction with nonce {} already pending for {}", nonce, sender));
            }
        }
        
        // Add to by_hash
        self.by_hash.insert(hash.clone(), tx);
        
        // Add to by_sender
        self.by_sender
            .entry(sender)
            .or_insert_with(std::collections::BTreeMap::new)
            .insert(nonce, hash);
        
        self.count += 1;
        Ok(true)
    }
    
    /// Remove transaction by hash
    pub fn remove(&mut self, hash: &str) -> Option<Transaction> {
        if let Some(tx) = self.by_hash.remove(hash) {
            if let Some(sender_txs) = self.by_sender.get_mut(&tx.from) {
                sender_txs.remove(&tx.nonce);
                if sender_txs.is_empty() {
                    self.by_sender.remove(&tx.from);
                }
            }
            self.count -= 1;
            Some(tx)
        } else {
            None
        }
    }
    
    /// Get transactions ready for block (sorted by sender, then nonce)
    pub fn get_pending(&self, max: usize) -> Vec<Transaction> {
        let mut result = Vec::new();
        
        // Collect all transactions
        for tx in self.by_hash.values() {
            result.push(tx.clone());
        }
        
        // Sort by (sender, nonce) to ensure correct ordering
        result.sort_by(|a, b| {
            match a.from.cmp(&b.from) {
                std::cmp::Ordering::Equal => a.nonce.cmp(&b.nonce),
                other => other,
            }
        });
        
        result.truncate(max);
        result
    }
    
    /// Drain transactions for block (removes them from mempool)
    pub fn drain_for_block(&mut self, max: usize) -> Vec<Transaction> {
        let txs = self.get_pending(max);
        for tx in &txs {
            self.remove(&tx.hash);
        }
        txs
    }
    
    /// Check if transaction exists
    pub fn contains(&self, hash: &str) -> bool {
        self.by_hash.contains_key(hash)
    }
    
    /// Get pending count
    pub fn len(&self) -> usize {
        self.count
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Get all pending transaction hashes
    pub fn get_hashes(&self) -> Vec<String> {
        self.by_hash.keys().cloned().collect()
    }
    
    /// Get pending transactions for an address
    pub fn get_by_sender(&self, sender: &str) -> Vec<Transaction> {
        if let Some(sender_txs) = self.by_sender.get(sender) {
            sender_txs.values()
                .filter_map(|hash| self.by_hash.get(hash).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get pending nonce for address (next expected nonce)
    /// Returns the higher of: confirmed_nonce or (max_pending_nonce + 1)
    pub fn get_pending_nonce(&self, sender: &str, confirmed_nonce: u64) -> u64 {
        if let Some(sender_txs) = self.by_sender.get(sender) {
            if let Some((&max_nonce, _)) = sender_txs.iter().last() {
                // Return the higher of confirmed nonce or pending nonce + 1
                return std::cmp::max(confirmed_nonce, max_nonce + 1);
            }
        }
        confirmed_nonce
    }
    
    /// Check if a specific sender+nonce is already pending
    pub fn has_pending_nonce(&self, sender: &str, nonce: u64) -> bool {
        if let Some(sender_txs) = self.by_sender.get(sender) {
            return sender_txs.contains_key(&nonce);
        }
        false
    }
}

pub struct Blockchain {
    pub config: Config,
    pub state: Arc<RwLock<State>>,
    pub mempool: Mempool,
    pub master_address: Address,
    pub mvm: MVM,
}

impl Blockchain {
    pub async fn new(
        config: Config,
        state: Arc<RwLock<State>>,
        master_address: Address,
    ) -> Result<Self, BoxError> {
        let mvm = MVM::new();
        
        let needs_genesis = {
            let state_guard = state.read().await;
            state_guard.get_block(0)?.is_none()
        };

        if needs_genesis {
            let genesis = Block::genesis(
                master_address.as_str(),
                config.genesis.master_balance * 100_000_000,
            );
            
            let mut state_guard = state.write().await;
            state_guard.save_block(&genesis)?;
            state_guard.set_balance(
                master_address.as_str(),
                config.genesis.master_balance * 100_000_000,
            )?;
            state_guard.set_height(0)?;
            
            tracing::info!("🌍 Genesis block created");
            tracing::info!("💰 Master balance: {} MVM", config.genesis.master_balance);
        }

        Ok(Blockchain {
            config,
            state,
            mempool: Mempool::new(),
            master_address,
            mvm,
        })
    }

    pub async fn produce_block(&mut self) -> Result<Block, BoxError> {
        let state_guard = self.state.read().await;
        let current_height = state_guard.get_height()?;
        let prev_block = state_guard.get_block(current_height)?.unwrap();
        drop(state_guard);

        // Get transactions from mempool (properly ordered by sender+nonce)
        let txs = self.mempool.drain_for_block(self.config.block.max_txs_per_block);
        
        tracing::debug!("📦 Processing {} transactions from mempool", txs.len());

        let mut executed_txs = Vec::new();
        for mut tx in txs {
            match self.execute_transaction(&mut tx).await {
                Ok(_) => {
                    tx.status = TxStatus::Success;
                    tracing::debug!("✅ TX {} success", &tx.hash[..8]);
                }
                Err(e) => {
                    tx.status = TxStatus::Failed;
                    tx.error = Some(e.to_string());
                    tracing::debug!("❌ TX {} failed: {}", &tx.hash[..8], e);
                }
            }
            executed_txs.push(tx);
        }

        let block_reward = self.config.rewards.block_reward * 100_000_000;
        let validator_reward = (block_reward * self.config.rewards.validator_percent) / 100;
        
        let rewards = BlockRewards {
            validator_reward,
            service_rewards: vec![],
            total_minted: block_reward,
        };

        let new_height = current_height + 1;
        let block = Block::new(
            new_height,
            &prev_block.hash,
            self.master_address.as_str(),
            executed_txs,
            rewards.clone(),
            self.config.block.gas_limit,
        );

        let mut state_guard = self.state.write().await;
        state_guard.save_block(&block)?;
        state_guard.set_height(new_height)?;
        
        // Index transactions for address lookup
        for tx in &block.transactions {
            state_guard.index_transaction(tx, new_height)?;
        }
        
        let current_balance = state_guard.get_balance(self.master_address.as_str())?;
        state_guard.set_balance(
            self.master_address.as_str(),
            current_balance + validator_reward,
        )?;

        let current_supply = state_guard.get_total_supply()?;
        state_guard.set_total_supply(current_supply + rewards.total_minted)?;

        Ok(block)
    }

    async fn execute_transaction(&mut self, tx: &mut Transaction) -> Result<(), TxError> {
        // Set gas based on tx type
        tx.gas_used = match &tx.tx_type {
            TxType::Transfer => 21000,
            TxType::Deploy => 200000,
            TxType::Call => 50000,
            TxType::CreateToken => 100000,
            TxType::TransferToken => 65000,
            TxType::DeployContract => 150000,
            TxType::CallContract => 50000,  // Base, actual depends on method
        };

        // Verify signature
        match tx.verify_signature() {
            Ok(true) => {},
            Ok(false) => return Err(TxError::InvalidSignature { 
                message: "Signature does not match sender address".to_string() 
            }),
            Err(e) => return Err(TxError::InvalidSignature { 
                message: e.to_string() 
            }),
        }

        // Verify nonce
        let expected_nonce = {
            let state_guard = self.state.read().await;
            state_guard.get_nonce(&tx.from).unwrap_or(0)
        };

        if tx.nonce != expected_nonce {
            return Err(TxError::InvalidNonce { expected: expected_nonce, got: tx.nonce });
        }

        // Calculate gas fee
        let gas_fee = tx.gas_used * tx.gas_price;

        // Check balance for gas fee (+ value for transfers)
        let total_cost = match &tx.tx_type {
            TxType::Transfer => tx.value + gas_fee,
            _ => gas_fee,
        };

        {
            let state_guard = self.state.read().await;
            let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
            if from_balance < total_cost {
                return Err(TxError::InsufficientBalance { required: total_cost, available: from_balance });
            }
        }

        // Execute transaction based on type
        match &tx.tx_type {
            TxType::Transfer => {
                let mut state_guard = self.state.write().await;
                let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;

                let to = tx.to.as_ref().ok_or_else(|| TxError::InvalidRecipient { 
                    message: "Missing recipient address".to_string() 
                })?;
                
                let to_addr = Address::new(to);
                if !to_addr.is_valid() {
                    return Err(TxError::InvalidAddress { address: to.clone() });
                }

                let to_balance = state_guard.get_balance(to).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                
                // Deduct value + gas fee from sender
                state_guard.set_balance(&tx.from, from_balance - total_cost).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                // Add value to recipient
                state_guard.set_balance(to, to_balance + tx.value).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
            }
            TxType::Deploy => {
                let mut state_guard = self.state.write().await;
                let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                
                // Deduct gas fee
                state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
            }
            TxType::Call => {
                if let Some(TxData::Call { contract, method, args }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deduct gas fee
                    state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    self.mvm.execute_call(&mut state_guard, contract, method, args)
                        .map_err(|e| TxError::ContractError { message: e.to_string() })?;
                    state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                }
            }
            TxType::CreateToken => {
                if let Some(TxData::CreateToken { name, symbol, total_supply }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deduct gas fee
                    state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    let contract_address = crate::standards::create_mvm20_token(
                        &mut state_guard,
                        &tx.from,
                        name,
                        symbol,
                        *total_supply,
                    ).map_err(|e| TxError::ContractError { message: e.to_string() })?;
                    tx.to = Some(contract_address);
                    state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                }
            }
            TxType::TransferToken => {
                if let Some(TxData::TransferToken { contract, to, amount }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deduct gas fee
                    state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Check token exists
                    let token = state_guard.get_token(contract)
                        .map_err(|e| TxError::InternalError { message: e.to_string() })?
                        .ok_or_else(|| TxError::TokenNotFound { contract: contract.clone() })?;
                    
                    // Check token balance
                    let token_balance = state_guard.get_token_balance(contract, &tx.from)
                        .map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    if token_balance < *amount {
                        return Err(TxError::InsufficientTokenBalance { required: *amount, available: token_balance });
                    }
                    
                    // Validate recipient
                    let to_addr = Address::new(to);
                    if !to_addr.is_valid() {
                        return Err(TxError::InvalidAddress { address: to.clone() });
                    }

                    crate::standards::transfer_mvm20(
                        &mut state_guard,
                        contract,
                        &tx.from,
                        to,
                        *amount,
                    ).map_err(|e| TxError::ContractError { message: e.to_string() })?;
                    
                    state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    drop(token);
                }
            }
            TxType::DeployContract => {
                if let Some(TxData::DeployContract { name, token, variables, mappings, functions }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deduct gas fee
                    state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deploy Mosh contract
                    let contract_addr = self.mvm.deploy(
                        &mut state_guard,
                        &tx.from,
                        name,
                        token.clone(),
                        variables.clone(),
                        mappings.clone(),
                        functions.clone(),
                    ).map_err(|e| TxError::ContractError { message: e.to_string() })?;
                    
                    tx.to = Some(contract_addr);
                    state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                }
            }
            TxType::CallContract => {
                if let Some(TxData::CallContract { contract, method, args, amount }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let from_balance = state_guard.get_balance(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Deduct base gas fee
                    state_guard.set_balance(&tx.from, from_balance - gas_fee).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                    
                    // Call Mosh contract
                    let result = self.mvm.call(
                        &mut state_guard,
                        &tx.from,
                        contract,
                        method,
                        args.clone(),
                        amount.unwrap_or(0),
                    ).map_err(|e| TxError::ContractError { message: e.to_string() })?;
                    
                    tx.gas_used = result.gas_used;
                    
                    if !result.success {
                        return Err(TxError::ContractError { 
                            message: result.error.unwrap_or("Unknown error".to_string())
                        });
                    }
                    
                    tx.to = Some(contract.clone());
                    state_guard.increment_nonce(&tx.from).map_err(|e| TxError::InternalError { message: e.to_string() })?;
                }
            }
        }

        Ok(())
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> Result<String, BoxError> {
        let hash = tx.hash.clone();
        
        // Add to mempool (handles duplicate checking)
        match self.mempool.add(tx) {
            Ok(true) => {
                tracing::debug!("📥 TX {} added to mempool (total: {})", &hash[..8], self.mempool.len());
                Ok(hash)
            }
            Ok(false) => {
                Err("Transaction already in mempool (duplicate hash)".into())
            }
            Err(e) => {
                Err(e.into())
            }
        }
    }
    
    /// Get pending transactions count
    pub fn pending_count(&self) -> usize {
        self.mempool.len()
    }
    
    /// Get pending transactions for address
    pub fn get_pending_txs(&self, address: &str) -> Vec<Transaction> {
        self.mempool.get_by_sender(address)
    }
    
    /// Get pending nonce (for next transaction)
    pub async fn get_pending_nonce(&self, address: &str) -> Result<u64, BoxError> {
        let confirmed_nonce = {
            let state_guard = self.state.read().await;
            state_guard.get_nonce(address)?
        };
        Ok(self.mempool.get_pending_nonce(address, confirmed_nonce))
    }

    pub async fn get_balance(&self, address: &str) -> Result<u64, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_balance(address)?)
    }

    pub async fn get_nonce(&self, address: &str) -> Result<u64, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_nonce(address)?)
    }

    pub async fn get_height(&self) -> Result<u64, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_height()?)
    }

    pub async fn get_block(&self, height: u64) -> Result<Option<Block>, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_block(height)?)
    }
}