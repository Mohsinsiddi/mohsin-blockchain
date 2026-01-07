use crate::chain::{Blockchain, Transaction, TxType, TxData, TxStatus, BoxError};
use crate::config::Config;
use crate::state::State;
use crate::network::{Network, StarNetwork};
use crate::address::{Address, hash_tx_data, verify_tx_signature};

use axum::{
    extract::{Path, State as AxumState, WebSocketUpgrade, ws::{WebSocket, Message}},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;
use chrono::Utc;

type SharedState = Arc<AppState>;

struct AppState {
    config: Config,
    blockchain: Arc<RwLock<Blockchain>>,
    state: Arc<RwLock<State>>,
    network: Arc<RwLock<StarNetwork>>,
}

pub async fn start_api_server(
    config: Config,
    blockchain: Arc<RwLock<Blockchain>>,
    state: Arc<RwLock<State>>,
    network: Arc<RwLock<StarNetwork>>,
) -> Result<(), BoxError> {
    let app_state = Arc::new(AppState {
        config: config.clone(),
        blockchain,
        state,
        network,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(get_status))
        .route("/block/:height", get(get_block))
        .route("/block/latest", get(get_latest_block))
        .route("/tx/:hash", get(get_transaction))
        .route("/balance/:address", get(get_balance))
        .route("/nonce/:address", get(get_nonce))
        .route("/account/:address", get(get_account))
        .route("/txs/:address", get(get_address_transactions))
        .route("/faucet/:address", post(faucet))
        .route("/tx", post(submit_transaction))
        .route("/tx/sign", post(sign_transaction))
        .route("/tokens", get(get_tokens))
        .route("/tokens/creator/:address", get(get_tokens_by_creator))
        .route("/tokens/holder/:address", get(get_token_holdings))
        .route("/token/:address", get(get_token))
        .route("/token/:contract/balance/:address", get(get_token_balance))
        .route("/contracts", get(get_contracts))
        .route("/contracts/creator/:address", get(get_contracts_by_creator))
        .route("/contract/:address", get(get_contract))
        .route("/contract/:address/mapping/:name", get(get_contract_mapping))
        .route("/wallet/new", get(create_wallet))
        .route("/ws", get(ws_handler))
        .route("/p2p", get(p2p_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = format!("{}:{}", config.network.host, config.network.api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn index() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "MOHSIN VIRTUAL MACHINE",
        "version": "0.3.0",
        "language": "Mosh",
        "endpoints": {
            "status": "GET /status",
            "block": "GET /block/:height",
            "latest_block": "GET /block/latest",
            "transaction": "GET /tx/:hash",
            "balance": "GET /balance/:address",
            "nonce": "GET /nonce/:address",
            "account": "GET /account/:address",
            "address_txs": "GET /txs/:address",
            "faucet": "POST /faucet/:address",
            "submit_tx": "POST /tx",
            "sign_tx": "POST /tx/sign",
            "tokens": "GET /tokens",
            "tokens_by_creator": "GET /tokens/creator/:address",
            "tokens_by_holder": "GET /tokens/holder/:address",
            "token_info": "GET /token/:address",
            "token_balance": "GET /token/:contract/balance/:address",
            "contracts": "GET /contracts",
            "contracts_by_creator": "GET /contracts/creator/:address",
            "contract_info": "GET /contract/:address",
            "contract_mapping": "GET /contract/:address/mapping/:name",
            "create_wallet": "GET /wallet/new",
            "websocket": "WS /ws",
            "p2p": "WS /p2p"
        },
        "tx_types": [
            "transfer",
            "create_token",
            "transfer_token",
            "deploy_contract",
            "call_contract"
        ],
        "contract_features": {
            "variables": ["uint64", "string", "bool", "address"],
            "mappings": "mapping(key_type => value_type)",
            "functions": {
                "modifiers": ["view", "write", "payable", "onlyOwner"],
                "operations": ["set", "add", "sub", "map_set", "map_add", "map_sub", "require", "transfer", "return", "let"]
            },
            "auto_methods": ["get_<var>", "set_<var>", "get_owner", "set_owner"]
        }
    }))
}

#[derive(Serialize)]
struct StatusResponse {
    chain_id: String,
    chain_name: String,
    height: u64,
    total_supply: String,
    peers: usize,
    browsers: usize,
    node_type: String,
}

async fn get_status(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let height = state_guard.get_height().unwrap_or(0);
    let total_supply = state_guard.get_total_supply().unwrap_or(0);
    drop(state_guard);

    let network = state.network.read().await;
    let peers = network.peer_count();
    let browsers = network.browser_count();
    drop(network);

    Json(StatusResponse {
        chain_id: state.config.chain.chain_id.clone(),
        chain_name: state.config.chain.chain_name.clone(),
        height,
        total_supply: format_balance(total_supply),
        peers,
        browsers,
        node_type: state.config.node.node_type.clone(),
    })
}

async fn get_block(
    Path(height): Path<u64>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_block(height) {
        Ok(Some(block)) => Json(serde_json::json!({ "success": true, "block": block })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ 
            "success": false, 
            "error": "block_not_found",
            "message": format!("Block {} not found", height)
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
    }
}

async fn get_latest_block(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let height = state_guard.get_height().unwrap_or(0);
    match state_guard.get_block(height) {
        Ok(Some(block)) => Json(serde_json::json!({ "success": true, "block": block })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ 
            "success": false,
            "error": "block_not_found",
            "message": "Latest block not found"
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
    }
}

async fn get_transaction(
    Path(hash): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_transaction(&hash) {
        Ok(Some(tx)) => {
            let fee_paid = tx.gas_used * tx.gas_price;
            Json(serde_json::json!({ 
                "success": true,
                "transaction": {
                    "hash": tx.hash,
                    "tx_type": tx.tx_type,
                    "from": tx.from,
                    "to": tx.to,
                    "value": format_balance(tx.value),
                    "value_raw": tx.value,
                    "gas_price": tx.gas_price,
                    "gas_limit": tx.gas_limit,
                    "gas_used": tx.gas_used,
                    "fee_paid": format_balance(fee_paid),
                    "fee_paid_raw": fee_paid,
                    "nonce": tx.nonce,
                    "data": tx.data,
                    "timestamp": tx.timestamp,
                    "signature": tx.signature,
                    "public_key": tx.public_key,
                    "status": tx.status,
                    "error": tx.error
                }
            })).into_response()
        },
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ 
            "success": false,
            "error": "tx_not_found",
            "message": format!("Transaction {} not found", hash)
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
    }
}

async fn get_balance(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    // Validate address
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let balance = state_guard.get_balance(&address).unwrap_or(0);
    
    Json(serde_json::json!({
        "success": true,
        "address": address,
        "balance": format_balance(balance),
        "balance_raw": balance
    })).into_response()
}

async fn get_nonce(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let nonce = state_guard.get_nonce(&address).unwrap_or(0);
    
    Json(serde_json::json!({
        "success": true,
        "address": address,
        "nonce": nonce
    })).into_response()
}

async fn get_account(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let balance = state_guard.get_balance(&address).unwrap_or(0);
    let nonce = state_guard.get_nonce(&address).unwrap_or(0);
    let tokens_created = state_guard.get_tokens_by_creator(&address).unwrap_or_default();
    let token_holdings = state_guard.get_token_holdings(&address).unwrap_or_default();
    let recent_txs = state_guard.get_transactions_by_address(&address, 20).unwrap_or_default();
    
    // Calculate total fees paid
    let total_fees_paid: u64 = recent_txs.iter()
        .filter(|tx| tx.from == address)
        .map(|tx| tx.gas_used * tx.gas_price)
        .sum();
    
    let txs_with_fees: Vec<serde_json::Value> = recent_txs.iter().map(|tx| {
        let fee_paid = tx.gas_used * tx.gas_price;
        serde_json::json!({
            "hash": tx.hash,
            "tx_type": tx.tx_type,
            "from": tx.from,
            "to": tx.to,
            "value": format_balance(tx.value),
            "value_raw": tx.value,
            "gas_used": tx.gas_used,
            "fee_paid": format_balance(fee_paid),
            "fee_paid_raw": fee_paid,
            "nonce": tx.nonce,
            "timestamp": tx.timestamp,
            "status": tx.status,
            "error": tx.error
        })
    }).collect();
    
    Json(serde_json::json!({
        "success": true,
        "account": {
            "address": address,
            "balance": format_balance(balance),
            "balance_raw": balance,
            "nonce": nonce,
            "total_fees_paid": format_balance(total_fees_paid),
            "total_fees_paid_raw": total_fees_paid,
            "tokens_created": tokens_created.len(),
            "tokens_held": token_holdings.len(),
            "tx_count": recent_txs.len()
        },
        "tokens_created": tokens_created,
        "token_holdings": token_holdings.iter().map(|h| serde_json::json!({
            "contract": h.contract,
            "name": h.name,
            "symbol": h.symbol,
            "balance": format_balance(h.balance),
            "balance_raw": h.balance
        })).collect::<Vec<_>>(),
        "recent_transactions": txs_with_fees
    })).into_response()
}

async fn get_address_transactions(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let txs = state_guard.get_transactions_by_address(&address, 100).unwrap_or_default();
    
    let txs_with_fees: Vec<serde_json::Value> = txs.iter().map(|tx| {
        let fee_paid = tx.gas_used * tx.gas_price;
        serde_json::json!({
            "hash": tx.hash,
            "tx_type": tx.tx_type,
            "from": tx.from,
            "to": tx.to,
            "value": format_balance(tx.value),
            "value_raw": tx.value,
            "gas_used": tx.gas_used,
            "fee_paid": format_balance(fee_paid),
            "fee_paid_raw": fee_paid,
            "nonce": tx.nonce,
            "timestamp": tx.timestamp,
            "status": tx.status,
            "error": tx.error
        })
    }).collect();
    
    Json(serde_json::json!({
        "success": true,
        "address": address,
        "count": txs_with_fees.len(),
        "transactions": txs_with_fees
    })).into_response()
}

async fn get_tokens_by_creator(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let tokens = state_guard.get_tokens_by_creator(&address).unwrap_or_default();
    
    Json(serde_json::json!({
        "success": true,
        "creator": address,
        "count": tokens.len(),
        "tokens": tokens
    })).into_response()
}

async fn get_token_holdings(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let holdings = state_guard.get_token_holdings(&address).unwrap_or_default();
    
    Json(serde_json::json!({
        "success": true,
        "address": address,
        "count": holdings.len(),
        "holdings": holdings.iter().map(|h| serde_json::json!({
            "contract": h.contract,
            "name": h.name,
            "symbol": h.symbol,
            "balance": format_balance(h.balance),
            "balance_raw": h.balance
        })).collect::<Vec<_>>()
    })).into_response()
}

// ===== MOSH CONTRACT ENDPOINTS =====

async fn get_contracts(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let contracts = state_guard.get_all_mosh_contracts().unwrap_or_default();
    
    Json(serde_json::json!({
        "success": true,
        "count": contracts.len(),
        "contracts": contracts.iter().map(|c| serde_json::json!({
            "address": c.address,
            "name": c.name,
            "creator": c.creator,
            "owner": c.owner,
            "token": c.token,
            "variables": c.variables.len(),
            "mappings": c.mappings.len(),
            "functions": c.functions.iter().map(|f| &f.name).collect::<Vec<_>>(),
            "created_at": c.created_at
        })).collect::<Vec<_>>()
    }))
}

async fn get_contracts_by_creator(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address: {}", address)
        }))).into_response();
    }

    let state_guard = state.state.read().await;
    let contracts = state_guard.get_mosh_contracts_by_creator(&address).unwrap_or_default();
    
    Json(serde_json::json!({
        "success": true,
        "creator": address,
        "count": contracts.len(),
        "contracts": contracts.iter().map(|c| serde_json::json!({
            "address": c.address,
            "name": c.name,
            "token": c.token,
            "functions": c.functions.len(),
            "created_at": c.created_at
        })).collect::<Vec<_>>()
    })).into_response()
}

async fn get_contract(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    
    match state_guard.get_mosh_contract(&address) {
        Ok(Some(c)) => {
            // Get current variable values
            let mut var_values = Vec::new();
            for var in &c.variables {
                let value = state_guard.get_mosh_var(&address, &var.name)
                    .unwrap_or(None)
                    .unwrap_or_default();
                var_values.push(serde_json::json!({
                    "name": var.name,
                    "type": format!("{:?}", var.var_type),
                    "value": value
                }));
            }
            
            // Get token info if linked
            let token_info = if let Some(ref token_addr) = c.token {
                state_guard.get_token(token_addr).ok().flatten().map(|t| serde_json::json!({
                    "address": t.address,
                    "name": t.name,
                    "symbol": t.symbol
                }))
            } else {
                None
            };
            
            // Build methods list
            let mut getters: Vec<String> = c.variables.iter().map(|v| format!("get_{}", v.name)).collect();
            let mut setters: Vec<String> = c.variables.iter().map(|v| format!("set_{}", v.name)).collect();
            
            // Add mapping getters/setters
            for m in &c.mappings {
                getters.push(format!("get_{}", m.name));
                setters.push(format!("set_{}", m.name));
            }
            
            // User functions
            let user_functions: Vec<serde_json::Value> = c.functions.iter().map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "modifiers": f.modifiers.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                    "args": f.args.iter().map(|a| serde_json::json!({
                        "name": a.name,
                        "type": format!("{:?}", a.arg_type)
                    })).collect::<Vec<_>>()
                })
            }).collect();
            
            Json(serde_json::json!({
                "success": true,
                "contract": {
                    "address": c.address,
                    "name": c.name,
                    "creator": c.creator,
                    "owner": c.owner,
                    "created_at": c.created_at,
                    "token": c.token,
                    "token_info": token_info
                },
                "variables": var_values,
                "mappings": c.mappings.iter().map(|m| serde_json::json!({
                    "name": m.name,
                    "key_type": format!("{:?}", m.key_type),
                    "value_type": format!("{:?}", m.value_type)
                })).collect::<Vec<_>>(),
                "functions": user_functions,
                "auto_methods": {
                    "getters": getters,
                    "setters": setters,
                    "reserved": ["get_owner", "set_owner", "get_creator", "get_token", "get_address"]
                }
            })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "success": false,
            "error": "contract_not_found",
            "message": format!("Contract not found: {}", address)
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "success": false,
            "error": "internal_error",
            "message": e.to_string()
        }))).into_response(),
    }
}

async fn get_contract_mapping(
    Path((address, map_name)): Path<(String, String)>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    
    match state_guard.get_mosh_contract(&address) {
        Ok(Some(c)) => {
            // Check mapping exists
            let mapping = c.mappings.iter().find(|m| m.name == map_name);
            if mapping.is_none() {
                return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                    "success": false,
                    "error": "mapping_not_found",
                    "message": format!("Mapping '{}' not found", map_name)
                }))).into_response();
            }
            
            let entries = state_guard.get_all_mosh_map_entries(&address, &map_name).unwrap_or_default();
            
            Json(serde_json::json!({
                "success": true,
                "contract": address,
                "mapping": map_name,
                "count": entries.len(),
                "entries": entries.iter().map(|(k, v)| serde_json::json!({
                    "key": k,
                    "value": v
                })).collect::<Vec<_>>()
            })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "success": false,
            "error": "contract_not_found",
            "message": format!("Contract not found: {}", address)
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "success": false,
            "error": "internal_error",
            "message": e.to_string()
        }))).into_response(),
    }
}

async fn create_wallet() -> impl IntoResponse {
    let keypair = crate::address::Keypair::generate();
    let address = keypair.address();
    let private_key = hex::encode(keypair.to_bytes());
    let public_key = keypair.public_key_hex();
    
    Json(serde_json::json!({
        "success": true,
        "address": address.as_str(),
        "public_key": public_key,
        "private_key": private_key,
        "warning": "Save your private key! It cannot be recovered."
    }))
}

async fn faucet(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    if !state.config.faucet.enabled {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ 
            "success": false,
            "error": "faucet_disabled",
            "message": "Faucet is disabled" 
        }))).into_response();
    }

    let addr = Address::new(&address);
    if !addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid address format: {}", address)
        }))).into_response();
    }

    let now = Utc::now().timestamp();
    let cooldown = state.config.faucet.cooldown as i64;
    let amount = state.config.faucet.amount * 100_000_000;

    let mut state_guard = state.state.write().await;
    
    if let Ok(Some(last_claim)) = state_guard.get_faucet_claim(&address) {
        if now - last_claim < cooldown {
            let remaining = cooldown - (now - last_claim);
            return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ 
                "success": false,
                "error": "cooldown_active",
                "message": format!("Faucet cooldown active. Try again in {} seconds", remaining),
                "remaining_seconds": remaining
            }))).into_response();
        }
    }

    let current_balance = state_guard.get_balance(&address).unwrap_or(0);
    if let Err(e) = state_guard.set_balance(&address, current_balance + amount) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response();
    }

    let _ = state_guard.set_faucet_claim(&address, now);

    Json(serde_json::json!({
        "success": true,
        "address": address,
        "amount": format_balance(amount),
        "new_balance": format_balance(current_balance + amount)
    })).into_response()
}

#[derive(Deserialize)]
struct SignTxRequest {
    private_key: String,
    tx_type: String,
    from: String,
    to: Option<String>,
    value: Option<u64>,
    nonce: u64,
    data: Option<serde_json::Value>,
}

async fn sign_transaction(
    Json(req): Json<SignTxRequest>,
) -> impl IntoResponse {
    // Load keypair from private key
    let keypair = match crate::address::Keypair::from_hex(&req.private_key) {
        Ok(kp) => kp,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_private_key",
            "message": e.to_string()
        }))).into_response(),
    };

    // Verify from address matches private key
    if keypair.address().as_str() != req.from {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "address_mismatch",
            "message": "Private key does not match 'from' address"
        }))).into_response();
    }

    // Convert data to TxData enum (same as submit does) for consistent hashing
    let tx_data: Option<TxData> = if let Some(ref d) = req.data {
        match req.tx_type.as_str() {
            "create_token" => Some(TxData::CreateToken {
                name: d["name"].as_str().unwrap_or("").to_string(),
                symbol: d["symbol"].as_str().unwrap_or("").to_string(),
                total_supply: d["total_supply"].as_u64().unwrap_or(0),
            }),
            "transfer_token" => Some(TxData::TransferToken {
                contract: d["contract"].as_str().unwrap_or("").to_string(),
                to: d["to"].as_str().unwrap_or("").to_string(),
                amount: d["amount"].as_u64().unwrap_or(0),
            }),
            "call" => Some(TxData::Call {
                contract: d["contract"].as_str().unwrap_or("").to_string(),
                method: d["method"].as_str().unwrap_or("").to_string(),
                args: d["args"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
            }),
            "deploy_contract" => {
                let variables: Vec<crate::mvm::VarDef> = d["variables"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| {
                        Some(crate::mvm::VarDef {
                            name: v["name"].as_str()?.to_string(),
                            var_type: crate::mvm::VarType::from_str(v["type"].as_str()?)?,
                            default: v["default"].as_str().map(|s| s.to_string()),
                        })
                    }).collect()).unwrap_or_default();
                let mappings: Vec<crate::mvm::MappingDef> = d["mappings"].as_array()
                    .map(|arr| arr.iter().filter_map(|m| {
                        Some(crate::mvm::MappingDef {
                            name: m["name"].as_str()?.to_string(),
                            key_type: crate::mvm::VarType::from_str(m["key_type"].as_str()?)?,
                            value_type: crate::mvm::VarType::from_str(m["value_type"].as_str()?)?,
                        })
                    }).collect()).unwrap_or_default();
                let functions: Vec<crate::mvm::FnDef> = d["functions"].as_array()
                    .map(|arr| arr.iter().filter_map(|f| {
                        Some(crate::mvm::FnDef {
                            name: f["name"].as_str()?.to_string(),
                            modifiers: f["modifiers"].as_array()
                                .map(|m| m.iter().filter_map(|x| match x.as_str()?.to_lowercase().as_str() {
                                    "view" => Some(crate::mvm::FnModifier::View),
                                    "write" => Some(crate::mvm::FnModifier::Write),
                                    "payable" => Some(crate::mvm::FnModifier::Payable),
                                    "onlyowner" | "only_owner" => Some(crate::mvm::FnModifier::OnlyOwner),
                                    _ => None,
                                }).collect()).unwrap_or_default(),
                            args: f["args"].as_array()
                                .map(|a| a.iter().filter_map(|x| Some(crate::mvm::FnArg {
                                    name: x["name"].as_str()?.to_string(),
                                    arg_type: crate::mvm::VarType::from_str(x["type"].as_str()?)?,
                                })).collect()).unwrap_or_default(),
                            body: f["body"].as_array()
                                .map(|b| b.iter().filter_map(|x| serde_json::from_value(x.clone()).ok()).collect())
                                .unwrap_or_default(),
                            returns: f["returns"].as_str().and_then(|s| crate::mvm::VarType::from_str(s)),
                        })
                    }).collect()).unwrap_or_default();
                Some(TxData::DeployContract {
                    name: d["name"].as_str().unwrap_or("").to_string(),
                    token: d["token"].as_str().map(|s| s.to_string()),
                    variables, mappings, functions,
                })
            },
            "call_contract" => Some(TxData::CallContract {
                contract: d["contract"].as_str().unwrap_or("").to_string(),
                method: d["method"].as_str().unwrap_or("").to_string(),
                args: d["args"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
                amount: d["amount"].as_u64(),
            }),
            _ => None
        }
    } else {
        None
    };

    let data_str = tx_data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
    let tx_hash = hash_tx_data(
        &req.tx_type,
        &req.from,
        req.to.as_deref(),
        req.value.unwrap_or(0) * 100_000_000,
        req.nonce,
        data_str.as_deref(),
    );

    let signature = keypair.sign_hex(&tx_hash);
    let public_key = keypair.public_key_hex();

    Json(serde_json::json!({
        "success": true,
        "tx_hash": hex::encode(&tx_hash),
        "signature": signature,
        "public_key": public_key,
        "message": "Use these values in the /tx endpoint"
    })).into_response()
}

#[derive(Deserialize)]
struct SubmitTxRequest {
    tx_type: String,
    from: String,
    to: Option<String>,
    value: Option<u64>,
    nonce: u64,
    data: Option<serde_json::Value>,
    signature: String,
    public_key: String,
}

async fn submit_transaction(
    AxumState(state): AxumState<SharedState>,
    Json(req): Json<SubmitTxRequest>,
) -> impl IntoResponse {
    // Validate from address
    let from_addr = Address::new(&req.from);
    if !from_addr.is_valid() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_address",
            "message": format!("Invalid 'from' address: {}", req.from)
        }))).into_response();
    }

    // Validate to address if present
    if let Some(ref to) = req.to {
        let to_addr = Address::new(to);
        if !to_addr.is_valid() {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": "invalid_address",
                "message": format!("Invalid 'to' address: {}", to)
            }))).into_response();
        }
    }

    // Parse tx_type
    let tx_type = match req.tx_type.as_str() {
        "transfer" => TxType::Transfer,
        "deploy" => TxType::Deploy,
        "call" => TxType::Call,
        "create_token" => TxType::CreateToken,
        "transfer_token" => TxType::TransferToken,
        "deploy_contract" => TxType::DeployContract,
        "call_contract" => TxType::CallContract,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ 
            "success": false,
            "error": "invalid_tx_type",
            "message": format!("Invalid transaction type: {}. Valid types: transfer, create_token, transfer_token, deploy_contract, call_contract", req.tx_type)
        }))).into_response(),
    };

    // Verify nonce
    let expected_nonce = {
        let state_guard = state.state.read().await;
        state_guard.get_nonce(&req.from).unwrap_or(0)
    };

    if req.nonce != expected_nonce {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_nonce",
            "message": format!("Invalid nonce: expected {}, got {}", expected_nonce, req.nonce),
            "expected_nonce": expected_nonce,
            "got_nonce": req.nonce
        }))).into_response();
    }

    // Parse data first (before signature verification)
    let data: Option<TxData> = if let Some(ref d) = req.data {
        match tx_type {
            TxType::CreateToken => {
                let name = d["name"].as_str().unwrap_or("").to_string();
                let symbol = d["symbol"].as_str().unwrap_or("").to_string();
                let total_supply = d["total_supply"].as_u64().unwrap_or(0);
                
                if name.is_empty() || symbol.is_empty() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Token name and symbol are required"
                    }))).into_response();
                }
                
                Some(TxData::CreateToken { name, symbol, total_supply })
            }
            TxType::TransferToken => {
                let contract = d["contract"].as_str().unwrap_or("").to_string();
                let to = d["to"].as_str().unwrap_or("").to_string();
                let amount = d["amount"].as_u64().unwrap_or(0);
                
                if contract.is_empty() || to.is_empty() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Contract address and recipient are required"
                    }))).into_response();
                }
                
                Some(TxData::TransferToken { contract, to, amount })
            }
            TxType::Call => {
                let contract = d["contract"].as_str().unwrap_or("").to_string();
                let method = d["method"].as_str().unwrap_or("").to_string();
                let args = d["args"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                
                if contract.is_empty() || method.is_empty() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Contract address and method name are required"
                    }))).into_response();
                }
                
                Some(TxData::Call { contract, method, args })
            }
            TxType::DeployContract => {
                let name = d["name"].as_str().unwrap_or("").to_string();
                let token = d["token"].as_str().map(|s| s.to_string());
                
                // Parse variables
                let variables: Vec<crate::mvm::VarDef> = d["variables"].as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|v| {
                            let name = v["name"].as_str()?.to_string();
                            let var_type = crate::mvm::VarType::from_str(v["type"].as_str()?)?;
                            let default = v["default"].as_str().map(|s| s.to_string());
                            Some(crate::mvm::VarDef { name, var_type, default })
                        }).collect()
                    })
                    .unwrap_or_default();
                
                // Parse mappings
                let mappings: Vec<crate::mvm::MappingDef> = d["mappings"].as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|m| {
                            let name = m["name"].as_str()?.to_string();
                            let key_type = crate::mvm::VarType::from_str(m["key_type"].as_str()?)?;
                            let value_type = crate::mvm::VarType::from_str(m["value_type"].as_str()?)?;
                            Some(crate::mvm::MappingDef { name, key_type, value_type })
                        }).collect()
                    })
                    .unwrap_or_default();
                
                // Parse functions
                let functions: Vec<crate::mvm::FnDef> = d["functions"].as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|f| {
                            let name = f["name"].as_str()?.to_string();
                            let modifiers: Vec<crate::mvm::FnModifier> = f["modifiers"].as_array()
                                .map(|mods| {
                                    mods.iter().filter_map(|m| {
                                        match m.as_str()?.to_lowercase().as_str() {
                                            "view" => Some(crate::mvm::FnModifier::View),
                                            "write" => Some(crate::mvm::FnModifier::Write),
                                            "payable" => Some(crate::mvm::FnModifier::Payable),
                                            "onlyowner" | "only_owner" => Some(crate::mvm::FnModifier::OnlyOwner),
                                            _ => None,
                                        }
                                    }).collect()
                                })
                                .unwrap_or_default();
                            let args: Vec<crate::mvm::FnArg> = f["args"].as_array()
                                .map(|args| {
                                    args.iter().filter_map(|a| {
                                        let name = a["name"].as_str()?.to_string();
                                        let arg_type = crate::mvm::VarType::from_str(a["type"].as_str()?)?;
                                        Some(crate::mvm::FnArg { name, arg_type })
                                    }).collect()
                                })
                                .unwrap_or_default();
                            let body: Vec<crate::mvm::Operation> = f["body"].as_array()
                                .map(|ops| {
                                    ops.iter().filter_map(|op| {
                                        serde_json::from_value(op.clone()).ok()
                                    }).collect()
                                })
                                .unwrap_or_default();
                            let returns = f["returns"].as_str()
                                .and_then(|s| crate::mvm::VarType::from_str(s));
                            Some(crate::mvm::FnDef { name, modifiers, args, body, returns })
                        }).collect()
                    })
                    .unwrap_or_default();
                
                if name.is_empty() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Contract name is required"
                    }))).into_response();
                }
                
                Some(TxData::DeployContract { name, token, variables, mappings, functions })
            }
            TxType::CallContract => {
                let contract = d["contract"].as_str().unwrap_or("").to_string();
                let method = d["method"].as_str().unwrap_or("").to_string();
                let args = d["args"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                let amount = d["amount"].as_u64();
                
                if contract.is_empty() || method.is_empty() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Contract address and method name are required"
                    }))).into_response();
                }
                
                Some(TxData::CallContract { contract, method, args, amount })
            }
            TxType::Transfer => {
                if req.to.is_none() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "success": false,
                        "error": "invalid_data",
                        "message": "Recipient address required for transfer"
                    }))).into_response();
                }
                None
            }
            _ => None
        }
    } else {
        if tx_type == TxType::Transfer && req.to.is_none() {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": "invalid_data",
                "message": "Recipient address required for transfer"
            }))).into_response();
        }
        None
    };

    // Verify signature using TxData serialization
    let data_str = data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
    let tx_hash = hash_tx_data(
        &req.tx_type,
        &req.from,
        req.to.as_deref(),
        req.value.unwrap_or(0) * 100_000_000,
        req.nonce,
        data_str.as_deref(),
    );

    match verify_tx_signature(&req.from, &tx_hash, &req.signature, &req.public_key) {
        Ok(true) => {},
        Ok(false) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "invalid_signature",
            "message": "Signature does not match sender address"
        }))).into_response(),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "success": false,
            "error": "signature_error",
            "message": format!("Error verifying signature: {}", e)
        }))).into_response(),
    }

    let mut tx = Transaction {
        hash: String::new(),
        tx_type,
        from: req.from,
        to: req.to,
        value: req.value.unwrap_or(0) * 100_000_000,
        gas_price: 1000,
        gas_limit: 100000,
        gas_used: 0,
        nonce: req.nonce,
        data,
        timestamp: Utc::now().timestamp(),
        signature: req.signature,
        public_key: req.public_key,
        status: TxStatus::Pending,
        error: None,
    };
    tx.hash = tx.calculate_hash();

    let mut blockchain = state.blockchain.write().await;
    match blockchain.add_transaction(tx.clone()) {
        Ok(hash) => {
            Json(serde_json::json!({
                "success": true,
                "hash": hash,
                "message": "Transaction submitted successfully"
            })).into_response()
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ 
                "success": false,
                "error": "tx_failed",
                "message": e.to_string() 
            }))).into_response()
        }
    }
}

async fn get_tokens(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_all_tokens() {
        Ok(tokens) => Json(serde_json::json!({ "success": true, "tokens": tokens })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
    }
}

async fn get_token(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_token(&address) {
        Ok(Some(token)) => Json(serde_json::json!({ "success": true, "token": token })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ 
            "success": false,
            "error": "token_not_found",
            "message": format!("Token not found: {}", address)
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
    }
}

async fn get_token_balance(
    Path((contract, address)): Path<(String, String)>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    
    // Check if token exists
    match state_guard.get_token(&contract) {
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ 
            "success": false,
            "error": "token_not_found",
            "message": format!("Token not found: {}", contract)
        }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ 
            "success": false,
            "error": "internal_error",
            "message": e.to_string() 
        }))).into_response(),
        Ok(Some(_)) => {}
    }

    let balance = state_guard.get_token_balance(&contract, &address).unwrap_or(0);
    
    Json(serde_json::json!({
        "success": true,
        "contract": contract,
        "address": address,
        "balance": format_balance(balance),
        "balance_raw": balance
    })).into_response()
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let config = state.config.clone();
    let db_state = state.state.clone();
    let network = state.network.clone();
    
    ws.on_upgrade(move |socket| handle_browser_socket(socket, config, db_state, network))
}

async fn handle_browser_socket(
    socket: WebSocket,
    config: Config,
    state: Arc<RwLock<State>>,
    network: Arc<RwLock<StarNetwork>>,
) {
    let (mut sender, mut receiver) = socket.split();
    
    let browser_id = uuid::Uuid::new_v4().to_string();
    info!("🌐 Browser connected: {}", &browser_id[..8]);

    let mut block_rx = {
        let net = network.read().await;
        net.subscribe_blocks()
    };

    let status = {
        let state_guard = state.read().await;
        let height = state_guard.get_height().unwrap_or(0);
        serde_json::json!({
            "type": "welcome",
            "height": height,
            "chain_id": config.chain.chain_id
        })
    };
    let _ = sender.send(Message::Text(status.to_string())).await;

    let broadcast_task = tokio::spawn(async move {
        while let Ok(block) = block_rx.recv().await {
            let msg = serde_json::json!({
                "type": "new_block",
                "block": block
            });
            if sender.send(Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(_text) = msg {
            // TODO: Handle browser queries
        }
    }

    broadcast_task.abort();
    info!("🌐 Browser disconnected: {}", &browser_id[..8]);
}

async fn p2p_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let network = state.network.clone();
    
    ws.on_upgrade(move |socket| async move {
        let peer_id = uuid::Uuid::new_v4().to_string();
        let (mut _sender, mut receiver) = socket.split();
        while let Some(Ok(_msg)) = receiver.next().await {
            // Handle messages
        }
        info!("🔌 P2P peer disconnected: {}", &peer_id[..8]);
        drop(network);
    })
}

fn format_balance(raw: u64) -> String {
    let whole = raw / 100_000_000;
    let fraction = raw % 100_000_000;
    if fraction > 0 {
        format!("{}.{:08}", whole, fraction)
    } else {
        whole.to_string()
    }
}