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

pub struct Blockchain {
    pub config: Config,
    pub state: Arc<RwLock<State>>,
    pub mempool: Vec<Transaction>,
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
            mempool: Vec::new(),
            master_address,
            mvm,
        })
    }

    pub async fn produce_block(&mut self) -> Result<Block, BoxError> {
        let state_guard = self.state.read().await;
        let current_height = state_guard.get_height()?;
        let prev_block = state_guard.get_block(current_height)?.unwrap();
        drop(state_guard);

        let txs: Vec<Transaction> = self.mempool
            .drain(..)
            .take(self.config.block.max_txs_per_block)
            .collect();

        let mut executed_txs = Vec::new();
        for mut tx in txs {
            match self.execute_transaction(&mut tx).await {
                Ok(_) => {
                    tx.status = TxStatus::Success;
                }
                Err(e) => {
                    tx.status = TxStatus::Failed;
                    tx.error = Some(e.to_string());
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
        self.mempool.push(tx);
        Ok(hash)
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