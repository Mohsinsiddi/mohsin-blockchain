//! MVM-20 Token Standard Implementation

use crate::state::State;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MVM20Token {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub creator: String,
    pub created_at: i64,
}

pub fn create_mvm20_token(
    state: &mut State,
    creator: &str,
    name: &str,
    symbol: &str,
    total_supply: u64,
) -> Result<String, BoxError> {
    let mut hasher = Sha256::new();
    hasher.update(creator);
    hasher.update(name);
    hasher.update(chrono::Utc::now().timestamp().to_le_bytes());
    let hash = hasher.finalize();
    let contract_address = format!("mvm1token{}", hex::encode(&hash[..10]));

    let token = MVM20Token {
        address: contract_address.clone(),
        name: name.to_string(),
        symbol: symbol.to_string(),
        decimals: 8,
        total_supply: total_supply * 100_000_000,
        creator: creator.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };

    state.save_token(&token)?;
    state.set_token_balance(&contract_address, creator, token.total_supply)?;

    tracing::info!("🪙 Token created: {} ({}) - Supply: {}", name, symbol, total_supply);

    Ok(contract_address)
}

pub fn transfer_mvm20(
    state: &mut State,
    contract: &str,
    from: &str,
    to: &str,
    amount: u64,
) -> Result<(), BoxError> {
    let from_balance = state.get_token_balance(contract, from)?;
    let to_balance = state.get_token_balance(contract, to)?;

    if from_balance < amount {
        return Err("Insufficient token balance".into());
    }

    state.set_token_balance(contract, from, from_balance - amount)?;
    state.set_token_balance(contract, to, to_balance + amount)?;

    Ok(())
}

pub fn balance_of_mvm20(
    state: &State,
    contract: &str,
    address: &str,
) -> Result<u64, BoxError> {
    Ok(state.get_token_balance(contract, address)?)
}

pub fn get_token_info(
    state: &State,
    contract: &str,
) -> Result<Option<MVM20Token>, BoxError> {
    state.get_token(contract)
}

pub fn get_all_tokens(
    state: &State,
) -> Result<Vec<MVM20Token>, BoxError> {
    state.get_all_tokens()
}
