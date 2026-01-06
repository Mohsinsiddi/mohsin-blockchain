use crate::chain::Block;
use crate::address::{Address, Keypair};
use crate::standards::MVM20Token;

use rocksdb::{DB, Options};
use serde::{Deserialize, Serialize};
use std::path::Path;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct State {
    db: DB,
    keypair: Option<Keypair>,
}

impl State {
    pub fn new(data_dir: &str) -> Result<Self, BoxError> {
        let path = Path::new(data_dir).join("rocksdb");
        std::fs::create_dir_all(&path)?;
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(100);
        
        let db = DB::open(&opts, path)?;
        
        Ok(State { db, keypair: None })
    }

    pub fn get_or_create_master_address(&mut self) -> Result<Address, BoxError> {
        if let Some(bytes) = self.db.get(b"meta:keypair")? {
            let key_bytes: [u8; 32] = bytes.as_slice().try_into()
                .map_err(|_| BoxError::from("Invalid keypair bytes"))?;
            let keypair = Keypair::from_bytes(&key_bytes)?;
            let address = keypair.address();
            self.keypair = Some(keypair);
            return Ok(address);
        }

        let keypair = Keypair::generate();
        let address = keypair.address();
        
        self.db.put(b"meta:keypair", keypair.to_bytes())?;
        self.keypair = Some(keypair);

        Ok(address)
    }

    pub fn get_keypair(&self) -> Option<&Keypair> {
        self.keypair.as_ref()
    }

    pub fn save_block(&mut self, block: &Block) -> Result<(), BoxError> {
        let key = format!("block:{}", block.height);
        let value = serde_json::to_string(block)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        
        let hash_key = format!("block_hash:{}", block.hash);
        self.db.put(hash_key.as_bytes(), block.height.to_le_bytes())?;
        
        for (idx, tx) in block.transactions.iter().enumerate() {
            let tx_key = format!("tx:{}", tx.hash);
            let tx_value = serde_json::to_string(tx)?;
            self.db.put(tx_key.as_bytes(), tx_value.as_bytes())?;
            
            let idx_key = format!("tx_by_block:{}:{}", block.height, idx);
            self.db.put(idx_key.as_bytes(), tx.hash.as_bytes())?;
        }

        Ok(())
    }

    pub fn get_block(&self, height: u64) -> Result<Option<Block>, BoxError> {
        let key = format!("block:{}", height);
        if let Some(value) = self.db.get(key.as_bytes())? {
            let block: Block = serde_json::from_slice(&value)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    pub fn get_block_by_hash(&self, hash: &str) -> Result<Option<Block>, BoxError> {
        let hash_key = format!("block_hash:{}", hash);
        if let Some(height_bytes) = self.db.get(hash_key.as_bytes())? {
            let height = u64::from_le_bytes(
                height_bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid height bytes"))?
            );
            self.get_block(height)
        } else {
            Ok(None)
        }
    }

    pub fn set_height(&mut self, height: u64) -> Result<(), BoxError> {
        self.db.put(b"meta:height", height.to_le_bytes())?;
        Ok(())
    }

    pub fn get_height(&self) -> Result<u64, BoxError> {
        if let Some(bytes) = self.db.get(b"meta:height")? {
            Ok(u64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid height bytes"))?
            ))
        } else {
            Ok(0)
        }
    }

    pub fn set_balance(&mut self, address: &str, balance: u64) -> Result<(), BoxError> {
        let key = format!("balance:{}", address);
        self.db.put(key.as_bytes(), balance.to_le_bytes())?;
        Ok(())
    }

    pub fn get_balance(&self, address: &str) -> Result<u64, BoxError> {
        let key = format!("balance:{}", address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(u64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid balance bytes"))?
            ))
        } else {
            Ok(0)
        }
    }

    pub fn set_total_supply(&mut self, supply: u64) -> Result<(), BoxError> {
        self.db.put(b"meta:total_supply", supply.to_le_bytes())?;
        Ok(())
    }

    pub fn get_total_supply(&self) -> Result<u64, BoxError> {
        if let Some(bytes) = self.db.get(b"meta:total_supply")? {
            Ok(u64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid supply bytes"))?
            ))
        } else {
            Ok(0)
        }
    }

    pub fn set_contract_storage(&mut self, contract: &str, key: &str, value: &str) -> Result<(), BoxError> {
        let storage_key = format!("storage:{}:{}", contract, key);
        self.db.put(storage_key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_contract_storage(&self, contract: &str, key: &str) -> Result<Option<String>, BoxError> {
        let storage_key = format!("storage:{}:{}", contract, key);
        if let Some(bytes) = self.db.get(storage_key.as_bytes())? {
            Ok(Some(String::from_utf8(bytes.to_vec())?))
        } else {
            Ok(None)
        }
    }

    pub fn save_token(&mut self, token: &MVM20Token) -> Result<(), BoxError> {
        let key = format!("token:{}", token.address);
        let value = serde_json::to_string(token)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        
        let list_key = format!("token_list:{}", token.address);
        self.db.put(list_key.as_bytes(), b"1")?;
        
        Ok(())
    }

    pub fn get_token(&self, address: &str) -> Result<Option<MVM20Token>, BoxError> {
        let key = format!("token:{}", address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            let token: MVM20Token = serde_json::from_slice(&bytes)?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_tokens(&self) -> Result<Vec<MVM20Token>, BoxError> {
        let mut tokens = Vec::new();
        let prefix = b"token:";
        
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with("token:") && !key_str.contains("_") && !key_str.contains("list") {
                let token: MVM20Token = serde_json::from_slice(&value)?;
                tokens.push(token);
            }
        }
        
        Ok(tokens)
    }

    pub fn set_token_balance(&mut self, contract: &str, address: &str, balance: u64) -> Result<(), BoxError> {
        let key = format!("token_balance:{}:{}", contract, address);
        self.db.put(key.as_bytes(), balance.to_le_bytes())?;
        Ok(())
    }

    pub fn get_token_balance(&self, contract: &str, address: &str) -> Result<u64, BoxError> {
        let key = format!("token_balance:{}:{}", contract, address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(u64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid token balance bytes"))?
            ))
        } else {
            Ok(0)
        }
    }

    pub fn save_node(&mut self, node_id: &str, data: &str) -> Result<(), BoxError> {
        let key = format!("node:{}", node_id);
        self.db.put(key.as_bytes(), data.as_bytes())?;
        Ok(())
    }

    pub fn get_node(&self, node_id: &str) -> Result<Option<String>, BoxError> {
        let key = format!("node:{}", node_id);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(Some(String::from_utf8(bytes.to_vec())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_faucet_claim(&self, address: &str) -> Result<Option<i64>, BoxError> {
        let key = format!("faucet:{}", address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(Some(i64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid faucet timestamp"))?
            )))
        } else {
            Ok(None)
        }
    }

    pub fn set_faucet_claim(&mut self, address: &str, timestamp: i64) -> Result<(), BoxError> {
        let key = format!("faucet:{}", address);
        self.db.put(key.as_bytes(), timestamp.to_le_bytes())?;
        Ok(())
    }

    pub fn get_state_snapshot(&self) -> Result<StateSnapshot, BoxError> {
        let height = self.get_height()?;
        let total_supply = self.get_total_supply()?;
        
        let mut balances = std::collections::HashMap::new();
        let prefix = b"balance:";
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(address) = key_str.strip_prefix("balance:") {
                let balance = u64::from_le_bytes(
                    value.as_ref().try_into()
                        .map_err(|_| BoxError::from("Invalid balance bytes"))?
                );
                balances.insert(address.to_string(), balance);
            }
        }

        let mut recent_blocks = Vec::new();
        let start = if height > 10 { height - 10 } else { 0 };
        for h in start..=height {
            if let Some(block) = self.get_block(h)? {
                recent_blocks.push(block);
            }
        }

        Ok(StateSnapshot {
            height,
            total_supply,
            balances,
            recent_blocks,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub height: u64,
    pub total_supply: u64,
    pub balances: std::collections::HashMap<String, u64>,
    pub recent_blocks: Vec<Block>,
}
