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
        .route("/faucet/:address", post(faucet))
        .route("/tx", post(submit_transaction))
        .route("/tx/sign", post(sign_transaction))
        .route("/tokens", get(get_tokens))
        .route("/token/:address", get(get_token))
        .route("/token/:contract/balance/:address", get(get_token_balance))
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
        "version": "0.1.0",
        "endpoints": {
            "status": "GET /status",
            "block": "GET /block/:height",
            "latest_block": "GET /block/latest",
            "transaction": "GET /tx/:hash",
            "balance": "GET /balance/:address",
            "nonce": "GET /nonce/:address",
            "faucet": "POST /faucet/:address",
            "submit_tx": "POST /tx",
            "sign_tx": "POST /tx/sign",
            "tokens": "GET /tokens",
            "token_info": "GET /token/:address",
            "token_balance": "GET /token/:contract/balance/:address",
            "create_wallet": "GET /wallet/new",
            "websocket": "WS /ws",
            "p2p": "WS /p2p"
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
    Path(_hash): Path<String>,
    AxumState(_state): AxumState<SharedState>,
) -> impl IntoResponse {
    Json(serde_json::json!({ 
        "success": false,
        "error": "not_implemented",
        "message": "Transaction lookup not implemented yet" 
    }))
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
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ 
            "success": false,
            "error": "invalid_tx_type",
            "message": format!("Invalid transaction type: {}. Valid types: transfer, deploy, call, create_token, transfer_token", req.tx_type)
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