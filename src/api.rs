use crate::chain::{Blockchain, Transaction, TxType, TxData, TxStatus, BoxError};
use crate::config::Config;
use crate::state::State;
use crate::network::{Network, StarNetwork};

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
        .route("/faucet/:address", post(faucet))
        .route("/tx", post(submit_transaction))
        .route("/tokens", get(get_tokens))
        .route("/token/:address", get(get_token))
        .route("/token/:contract/balance/:address", get(get_token_balance))
        .route("/ws", get(ws_handler))
        .route("/p2p", get(p2p_handler))
        .route("/wallet/new", get(create_wallet))
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
            "status": "/status",
            "block": "/block/:height",
            "latest_block": "/block/latest",
            "transaction": "/tx/:hash",
            "balance": "/balance/:address",
            "faucet": "/faucet/:address (POST)",
            "submit_tx": "/tx (POST)",
            "tokens": "/tokens",
            "websocket": "/ws",
            "p2p": "/p2p"
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
        Ok(Some(block)) => Json(serde_json::json!({ "block": block })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Block not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn get_latest_block(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let height = state_guard.get_height().unwrap_or(0);
    match state_guard.get_block(height) {
        Ok(Some(block)) => Json(serde_json::json!({ "block": block })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Block not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn get_transaction(
    Path(_hash): Path<String>,
    AxumState(_state): AxumState<SharedState>,
) -> impl IntoResponse {
    Json(serde_json::json!({ "error": "Not implemented yet" }))
}

async fn get_balance(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let balance = state_guard.get_balance(&address).unwrap_or(0);
    
    Json(serde_json::json!({
        "address": address,
        "balance": format_balance(balance),
        "balance_raw": balance
    }))
}

async fn faucet(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    if !state.config.faucet.enabled {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Faucet disabled" }))).into_response();
    }

    let now = Utc::now().timestamp();
    let cooldown = state.config.faucet.cooldown as i64;
    let amount = state.config.faucet.amount * 100_000_000;

    let mut state_guard = state.state.write().await;
    
    if let Ok(Some(last_claim)) = state_guard.get_faucet_claim(&address) {
        if now - last_claim < cooldown {
            let remaining = cooldown - (now - last_claim);
            return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({ 
                "error": "Cooldown active",
                "remaining_seconds": remaining
            }))).into_response();
        }
    }

    let current_balance = state_guard.get_balance(&address).unwrap_or(0);
    if let Err(e) = state_guard.set_balance(&address, current_balance + amount) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
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
struct SubmitTxRequest {
    tx_type: String,
    from: String,
    to: Option<String>,
    value: Option<u64>,
    data: Option<serde_json::Value>,
    signature: String,
}

async fn submit_transaction(
    AxumState(state): AxumState<SharedState>,
    Json(req): Json<SubmitTxRequest>,
) -> impl IntoResponse {
    let tx_type = match req.tx_type.as_str() {
        "transfer" => TxType::Transfer,
        "deploy" => TxType::Deploy,
        "call" => TxType::Call,
        "create_token" => TxType::CreateToken,
        "transfer_token" => TxType::TransferToken,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid tx_type" }))).into_response(),
    };

    let data = if let Some(d) = req.data {
        match tx_type {
            TxType::CreateToken => {
                Some(TxData::CreateToken {
                    name: d["name"].as_str().unwrap_or("").to_string(),
                    symbol: d["symbol"].as_str().unwrap_or("").to_string(),
                    total_supply: d["total_supply"].as_u64().unwrap_or(0),
                })
            }
            TxType::TransferToken => {
                Some(TxData::TransferToken {
                    contract: d["contract"].as_str().unwrap_or("").to_string(),
                    to: d["to"].as_str().unwrap_or("").to_string(),
                    amount: d["amount"].as_u64().unwrap_or(0),
                })
            }
            TxType::Call => {
                Some(TxData::Call {
                    contract: d["contract"].as_str().unwrap_or("").to_string(),
                    method: d["method"].as_str().unwrap_or("").to_string(),
                    args: d["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default(),
                })
            }
            _ => None
        }
    } else {
        None
    };

    let mut tx = Transaction {
        hash: String::new(),
        tx_type,
        from: req.from,
        to: req.to,
        value: req.value.unwrap_or(0) * 100_000_000,
        gas_price: 1000,
        gas_limit: 100000,
        gas_used: 0,
        nonce: 0,
        data,
        timestamp: Utc::now().timestamp(),
        signature: req.signature,
        status: TxStatus::Pending,
        error: None,
    };
    tx.hash = tx.calculate_hash();

    let mut blockchain = state.blockchain.write().await;
    match blockchain.add_transaction(tx.clone()) {
        Ok(hash) => {
            Json(serde_json::json!({
                "success": true,
                "hash": hash
            })).into_response()
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

async fn get_tokens(
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_all_tokens() {
        Ok(tokens) => Json(serde_json::json!({ "tokens": tokens })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn get_token(
    Path(address): Path<String>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    match state_guard.get_token(&address) {
        Ok(Some(token)) => Json(serde_json::json!({ "token": token })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Token not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn get_token_balance(
    Path((contract, address)): Path<(String, String)>,
    AxumState(state): AxumState<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.state.read().await;
    let balance = state_guard.get_token_balance(&contract, &address).unwrap_or(0);
    
    Json(serde_json::json!({
        "contract": contract,
        "address": address,
        "balance": format_balance(balance),
        "balance_raw": balance
    }))
}

async fn create_wallet() -> impl IntoResponse {
    let keypair = crate::address::Keypair::generate();
    let address = keypair.address();
    let private_key = hex::encode(keypair.to_bytes());
    
    Json(serde_json::json!({
        "address": address.as_str(),
        "private_key": private_key,
        "warning": "Save your private key! It cannot be recovered."
    }))
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
        drop(network); // Keep reference alive
    })
}

fn format_balance(raw: u64) -> String {
    let whole = raw / 100_000_000;
    let fraction = (raw % 100_000_000) / 1_000_000_000_000_000;
    if fraction > 0 {
        format!("{}.{:03}", whole, fraction)
    } else {
        whole.to_string()
    }
}