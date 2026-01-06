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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxData {
    Deploy { code: Vec<u8>, name: String },
    Call { contract: String, method: String, args: Vec<String> },
    CreateToken { name: String, symbol: String, total_supply: u64 },
    TransferToken { contract: String, to: String, amount: u64 },
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
        
        // Check if genesis exists
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

        // Get transactions from mempool
        let txs: Vec<Transaction> = self.mempool
            .drain(..)
            .take(self.config.block.max_txs_per_block)
            .collect();

        // Execute transactions
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

        // Calculate rewards
        let block_reward = self.config.rewards.block_reward * 100_000_000;
        let validator_reward = (block_reward * self.config.rewards.validator_percent) / 100;

        
        let rewards = BlockRewards {
            validator_reward,
            service_rewards: vec![],
            total_minted: block_reward,
        };

        // Create new block
        let new_height = current_height + 1;
        let block = Block::new(
            new_height,
            &prev_block.hash,
            self.master_address.as_str(),
            executed_txs,
            rewards.clone(),
            self.config.block.gas_limit,
        );

        // Save block and update state
        let mut state_guard = self.state.write().await;
        state_guard.save_block(&block)?;
        state_guard.set_height(new_height)?;
        
        // Credit validator reward
        let current_balance = state_guard.get_balance(self.master_address.as_str())?;
        state_guard.set_balance(
            self.master_address.as_str(),
            current_balance + validator_reward,
        )?;

        // Update total supply
        let current_supply = state_guard.get_total_supply()?;
        state_guard.set_total_supply(current_supply + rewards.total_minted)?;

        Ok(block)
    }

    async fn execute_transaction(&mut self, tx: &mut Transaction) -> Result<(), BoxError> {
        tx.gas_used = 21000;

        match &tx.tx_type {
            TxType::Transfer => {
                let mut state_guard = self.state.write().await;
                let from_balance = state_guard.get_balance(&tx.from)?;
                let total_cost = tx.value + (tx.gas_used * tx.gas_price);
                
                if from_balance < total_cost {
                    return Err("Insufficient balance".into());
                }

                let to = tx.to.as_ref().ok_or_else(|| BoxError::from("Missing recipient"))?;
                let to_balance = state_guard.get_balance(to)?;
                
                state_guard.set_balance(&tx.from, from_balance - total_cost)?;
                state_guard.set_balance(to, to_balance + tx.value)?;
            }
            TxType::Deploy => {
                tx.gas_used = 200000;
            }
            TxType::Call => {
                tx.gas_used = 50000;
                if let Some(TxData::Call { contract, method, args }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    self.mvm.execute_call(&mut state_guard, contract, method, args)?;
                }
            }
            TxType::CreateToken => {
                tx.gas_used = 100000;
                if let Some(TxData::CreateToken { name, symbol, total_supply }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    let contract_address = crate::standards::create_mvm20_token(
                        &mut state_guard,
                        &tx.from,
                        name,
                        symbol,
                        *total_supply,
                    )?;
                    tx.to = Some(contract_address);
                }
            }
            TxType::TransferToken => {
                tx.gas_used = 65000;
                if let Some(TxData::TransferToken { contract, to, amount }) = &tx.data {
                    let mut state_guard = self.state.write().await;
                    crate::standards::transfer_mvm20(
                        &mut state_guard,
                        contract,
                        &tx.from,
                        to,
                        *amount,
                    )?;
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

    pub async fn get_height(&self) -> Result<u64, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_height()?)
    }

    pub async fn get_block(&self, height: u64) -> Result<Option<Block>, BoxError> {
        let state_guard = self.state.read().await;
        Ok(state_guard.get_block(height)?)
    }
}
