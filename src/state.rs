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

    // Block operations
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

    // Height operations
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

    // Balance operations
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

    // Nonce operations
    pub fn set_nonce(&mut self, address: &str, nonce: u64) -> Result<(), BoxError> {
        let key = format!("nonce:{}", address);
        self.db.put(key.as_bytes(), nonce.to_le_bytes())?;
        Ok(())
    }

    pub fn get_nonce(&self, address: &str) -> Result<u64, BoxError> {
        let key = format!("nonce:{}", address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(u64::from_le_bytes(
                bytes.as_slice().try_into()
                    .map_err(|_| BoxError::from("Invalid nonce bytes"))?
            ))
        } else {
            Ok(0)
        }
    }

    pub fn increment_nonce(&mut self, address: &str) -> Result<u64, BoxError> {
        let current = self.get_nonce(address)?;
        let new_nonce = current + 1;
        self.set_nonce(address, new_nonce)?;
        Ok(new_nonce)
    }

    // Total supply
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

    // Contract storage
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

    // Contract variables (alias for storage)
    pub fn set_contract_var(&mut self, contract: &str, var_name: &str, value: &str) -> Result<(), BoxError> {
        self.set_contract_storage(contract, var_name, value)
    }

    pub fn get_contract_var(&self, contract: &str, var_name: &str) -> Result<Option<String>, BoxError> {
        self.get_contract_storage(contract, var_name)
    }

    // ==================== MOSH CONTRACTS ====================

    pub fn save_mosh_contract(&mut self, contract: &crate::mvm::MoshContract) -> Result<(), BoxError> {
        let key = format!("mosh:{}", contract.address);
        let value = serde_json::to_string(contract)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        
        let creator_key = format!("mosh_by_creator:{}:{}", contract.creator, contract.address);
        self.db.put(creator_key.as_bytes(), b"1")?;
        
        Ok(())
    }

    pub fn get_mosh_contract(&self, address: &str) -> Result<Option<crate::mvm::MoshContract>, BoxError> {
        let key = format!("mosh:{}", address);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            let contract: crate::mvm::MoshContract = serde_json::from_slice(&bytes)?;
            Ok(Some(contract))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_mosh_contracts(&self) -> Result<Vec<crate::mvm::MoshContract>, BoxError> {
        let mut contracts = Vec::new();
        let prefix = b"mosh:mvm1contract";
        
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with("mosh:mvm1contract") {
                let contract: crate::mvm::MoshContract = serde_json::from_slice(&value)?;
                contracts.push(contract);
            }
        }
        
        Ok(contracts)
    }

    pub fn get_mosh_contracts_by_creator(&self, creator: &str) -> Result<Vec<crate::mvm::MoshContract>, BoxError> {
        let mut contracts = Vec::new();
        let prefix = format!("mosh_by_creator:{}:", creator);
        
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(addr) = key_str.strip_prefix(&prefix) {
                if let Some(contract) = self.get_mosh_contract(addr)? {
                    contracts.push(contract);
                }
            }
        }
        
        Ok(contracts)
    }

    // ==================== MOSH VARIABLES ====================

    pub fn set_mosh_var(&mut self, contract: &str, var: &str, value: &str) -> Result<(), BoxError> {
        let key = format!("mosh_var:{}:{}", contract, var);
        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_mosh_var(&self, contract: &str, var: &str) -> Result<Option<String>, BoxError> {
        let key = format!("mosh_var:{}:{}", contract, var);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            Ok(Some(String::from_utf8(bytes.to_vec())?))
        } else {
            Ok(None)
        }
    }

    // ==================== MOSH MAPPINGS ====================

    pub fn set_mosh_map(&mut self, contract: &str, map: &str, key: &str, value: &str) -> Result<(), BoxError> {
        let db_key = format!("mosh_map:{}:{}:{}", contract, map, key);
        self.db.put(db_key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_mosh_map(&self, contract: &str, map: &str, key: &str) -> Result<Option<String>, BoxError> {
        let db_key = format!("mosh_map:{}:{}:{}", contract, map, key);
        if let Some(bytes) = self.db.get(db_key.as_bytes())? {
            Ok(Some(String::from_utf8(bytes.to_vec())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_mosh_map_entries(&self, contract: &str, map: &str) -> Result<Vec<(String, String)>, BoxError> {
        let mut entries = Vec::new();
        let prefix = format!("mosh_map:{}:{}:", contract, map);
        
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(map_key) = key_str.strip_prefix(&prefix) {
                let val = String::from_utf8(value.to_vec())?;
                entries.push((map_key.to_string(), val));
            }
        }
        
        Ok(entries)
    }

    // ==================== CONTRACT EVENTS ====================

    pub fn save_contract_event(&mut self, event: &crate::mvm::ContractEvent) -> Result<(), BoxError> {
        // Key: event:{contract}:{height}:{index}
        // Find next index for this contract+height
        let prefix = format!("event:{}:{}:", event.contract, event.block_height);
        let mut idx = 0u64;
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with(&prefix) {
                idx += 1;
            } else {
                break;
            }
        }

        let key = format!("event:{}:{}:{}", event.contract, event.block_height, idx);
        let value = serde_json::to_string(event)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_contract_events(&self, contract: &str) -> Result<Vec<crate::mvm::ContractEvent>, BoxError> {
        let mut events = Vec::new();
        let prefix = format!("event:{}:", contract);

        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with(&prefix) {
                let event: crate::mvm::ContractEvent = serde_json::from_slice(&value)?;
                events.push(event);
            } else {
                break;
            }
        }

        events.sort_by(|a, b| b.block_height.cmp(&a.block_height));
        Ok(events)
    }

    // ==================== LEADERBOARD ====================

    pub fn get_leaderboard(&self) -> Result<serde_json::Value, BoxError> {
        // Top balances
        let mut balances: Vec<(String, u64)> = Vec::new();
        let prefix = b"balance:";
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(addr) = key_str.strip_prefix("balance:") {
                if let Ok(bytes) = value.as_ref().try_into() {
                    let bal: u64 = u64::from_le_bytes(bytes);
                    if bal > 0 {
                        balances.push((addr.to_string(), bal));
                    }
                }
            }
        }
        balances.sort_by(|a, b| b.1.cmp(&a.1));
        balances.truncate(10);

        // Top token creators
        let all_tokens = self.get_all_tokens()?;
        let mut creator_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for token in &all_tokens {
            *creator_counts.entry(token.creator.clone()).or_insert(0) += 1;
        }
        let mut top_creators: Vec<(String, usize)> = creator_counts.into_iter().collect();
        top_creators.sort_by(|a, b| b.1.cmp(&a.1));
        top_creators.truncate(10);

        // Top contract deployers
        let all_contracts = self.get_all_mosh_contracts()?;
        let mut deployer_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for c in &all_contracts {
            *deployer_counts.entry(c.creator.clone()).or_insert(0) += 1;
        }
        let mut top_deployers: Vec<(String, usize)> = deployer_counts.into_iter().collect();
        top_deployers.sort_by(|a, b| b.1.cmp(&a.1));
        top_deployers.truncate(10);

        // Top transaction senders (by nonce as proxy for tx count)
        let mut tx_counts: Vec<(String, u64)> = Vec::new();
        let nonce_prefix = b"nonce:";
        let iter = self.db.prefix_iterator(nonce_prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(addr) = key_str.strip_prefix("nonce:") {
                if let Ok(bytes) = value.as_ref().try_into() {
                    let nonce: u64 = u64::from_le_bytes(bytes);
                    if nonce > 0 {
                        tx_counts.push((addr.to_string(), nonce));
                    }
                }
            }
        }
        tx_counts.sort_by(|a, b| b.1.cmp(&a.1));
        tx_counts.truncate(10);

        Ok(serde_json::json!({
            "top_balances": balances.iter().map(|(a, b)| serde_json::json!({"address": a, "balance": b, "formatted": format!("{}.{:08}", b / 100_000_000, b % 100_000_000)})).collect::<Vec<_>>(),
            "top_token_creators": top_creators.iter().map(|(a, c)| serde_json::json!({"address": a, "count": c})).collect::<Vec<_>>(),
            "top_contract_deployers": top_deployers.iter().map(|(a, c)| serde_json::json!({"address": a, "count": c})).collect::<Vec<_>>(),
            "top_tx_senders": tx_counts.iter().map(|(a, c)| serde_json::json!({"address": a, "count": c})).collect::<Vec<_>>(),
        }))
    }

    // Token operations (MVM-20)
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

    pub fn get_token_holders(&self, contract: &str) -> Result<Vec<(String, u64)>, BoxError> {
        let mut holders = Vec::new();
        let prefix = format!("token_balance:{}:", contract);

        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if let Some(address) = key_str.strip_prefix(&prefix) {
                let balance = u64::from_le_bytes(
                    value.as_ref().try_into().unwrap_or([0u8; 8])
                );
                if balance > 0 {
                    holders.push((address.to_string(), balance));
                }
            }
        }
        // Sort by balance descending
        holders.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(holders)
    }

    // Faucet operations
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

    // Transaction operations
    pub fn get_transaction(&self, hash: &str) -> Result<Option<crate::chain::Transaction>, BoxError> {
        let key = format!("tx:{}", hash);
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            let tx: crate::chain::Transaction = serde_json::from_slice(&bytes)?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    pub fn save_transaction(&mut self, tx: &crate::chain::Transaction) -> Result<(), BoxError> {
        let key = format!("tx:{}", tx.hash);
        let value = serde_json::to_string(tx)?;
        self.db.put(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn index_transaction(&mut self, tx: &crate::chain::Transaction, block_height: u64) -> Result<(), BoxError> {
        // Index by sender
        let from_key = format!("tx_by_addr:{}:{}", tx.from, tx.hash);
        self.db.put(from_key.as_bytes(), block_height.to_le_bytes())?;

        // Index by recipient if exists
        if let Some(ref to) = tx.to {
            let to_key = format!("tx_by_addr:{}:{}", to, tx.hash);
            self.db.put(to_key.as_bytes(), block_height.to_le_bytes())?;
        }

        // Index by contract/token address from tx data
        if let Some(ref data) = tx.data {
            let contract_addr = match data {
                crate::chain::TxData::TransferToken { contract, to, .. } => {
                    // Index by token contract AND by token recipient
                    let to_key = format!("tx_by_addr:{}:{}", to, tx.hash);
                    self.db.put(to_key.as_bytes(), block_height.to_le_bytes())?;
                    Some(contract.as_str())
                }
                crate::chain::TxData::CallContract { contract, .. } => Some(contract.as_str()),
                crate::chain::TxData::Call { contract, .. } => Some(contract.as_str()),
                _ => None,
            };
            if let Some(addr) = contract_addr {
                let contract_key = format!("tx_by_addr:{}:{}", addr, tx.hash);
                self.db.put(contract_key.as_bytes(), block_height.to_le_bytes())?;
            }
        }

        // Index tx hash → block height
        let block_key = format!("tx_block:{}", tx.hash);
        self.db.put(block_key.as_bytes(), block_height.to_le_bytes())?;

        Ok(())
    }

    pub fn get_transaction_block_height(&self, tx_hash: &str) -> Result<Option<u64>, BoxError> {
        let key = format!("tx_block:{}", tx_hash);
        match self.db.get(key.as_bytes())? {
            Some(bytes) => {
                let slice: &[u8] = &bytes;
                let arr: [u8; 8] = slice.try_into().unwrap_or([0u8; 8]);
                Ok(Some(u64::from_le_bytes(arr)))
            }
            None => Ok(None),
        }
    }

    pub fn get_transactions_by_address(&self, address: &str, limit: usize) -> Result<Vec<crate::chain::Transaction>, BoxError> {
        let mut txs = Vec::new();
        let prefix = format!("tx_by_addr:{}:", address);
        
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        for item in iter.take(limit) {
            let (key, _) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract tx hash from key
            if let Some(tx_hash) = key_str.strip_prefix(&prefix) {
                if let Some(tx) = self.get_transaction(tx_hash)? {
                    txs.push(tx);
                }
            }
        }
        
        // Sort by timestamp descending
        txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(txs)
    }

    // Token query operations
    pub fn get_tokens_by_creator(&self, creator: &str) -> Result<Vec<MVM20Token>, BoxError> {
        let mut tokens = Vec::new();
        let all_tokens = self.get_all_tokens()?;
        
        for token in all_tokens {
            if token.creator == creator {
                tokens.push(token);
            }
        }
        
        Ok(tokens)
    }

    pub fn get_token_holdings(&self, address: &str) -> Result<Vec<TokenHolding>, BoxError> {
        let mut holdings = Vec::new();
        let prefix = b"token_balance:";
        
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Key format: token_balance:CONTRACT:ADDRESS
            if let Some(rest) = key_str.strip_prefix("token_balance:") {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() == 2 && parts[1] == address {
                    let contract = parts[0].to_string();
                    let balance = u64::from_le_bytes(
                        value.as_ref().try_into()
                            .map_err(|_| BoxError::from("Invalid balance bytes"))?
                    );
                    
                    if balance > 0 {
                        // Get token info
                        if let Some(token) = self.get_token(&contract)? {
                            holdings.push(TokenHolding {
                                contract: contract.clone(),
                                name: token.name,
                                symbol: token.symbol,
                                balance,
                                decimals: token.decimals,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(holdings)
    }

    // State snapshot for sync
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolding {
    pub contract: String,
    pub name: String,
    pub symbol: String,
    pub balance: u64,
    pub decimals: u8,
}