use crate::chain::{Block, Blockchain, Transaction, BoxError};
use crate::config::Config;
use crate::state::{State, StateSnapshot};
use crate::network::Network;

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, mpsc};
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum P2PMessage {
    Hello { node_id: String, node_type: String },
    Welcome { node_id: String, height: u64, peers: Vec<String> },
    GetState,
    StateSnapshot(StateSnapshot),
    NewBlock(Block),
    GetBlock { height: u64 },
    BlockResponse(Option<Block>),
    SubmitTx(Transaction),
    TxConfirmed { hash: String },
    Ping,
    Pong,
}

#[derive(Clone)]
pub struct ConnectedPeer {
    pub node_id: String,
    pub node_type: String,
    pub tx: mpsc::Sender<P2PMessage>,
}

pub struct StarNetwork {
    config: Config,
    blockchain: Arc<RwLock<Blockchain>>,
    state: Arc<RwLock<State>>,
    peers: Arc<RwLock<HashMap<String, ConnectedPeer>>>,
    browsers: Arc<RwLock<HashMap<String, mpsc::Sender<P2PMessage>>>>,
    block_tx: broadcast::Sender<Block>,
}

impl StarNetwork {
    pub fn new(
        config: Config,
        blockchain: Arc<RwLock<Blockchain>>,
        state: Arc<RwLock<State>>,
    ) -> Self {
        let (block_tx, _) = broadcast::channel(100);
        
        StarNetwork {
            config,
            blockchain,
            state,
            peers: Arc::new(RwLock::new(HashMap::new())),
            browsers: Arc::new(RwLock::new(HashMap::new())),
            block_tx,
        }
    }

    pub async fn handle_peer_connection(
        self: Arc<Self>,
        ws: WebSocket,
        peer_id: String,
    ) {
        let (mut sender, mut receiver) = ws.split();
        let (tx, mut rx) = mpsc::channel::<P2PMessage>(100);

        // Send welcome message
        let height = {
            let state = self.state.read().await;
            state.get_height().unwrap_or(0)
        };
        
        let peers: Vec<String> = {
            let peers_guard = self.peers.read().await;
            peers_guard.keys().cloned().collect()
        };

        let welcome = P2PMessage::Welcome {
            node_id: self.config.node.id.clone(),
            height,
            peers,
        };

        if let Ok(msg) = serde_json::to_string(&welcome) {
            let _ = sender.send(Message::Text(msg)).await;
        }

        // Spawn sender task
        let sender_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Clone what we need for the message handler
        let peers = self.peers.clone();
        let blockchain = self.blockchain.clone();
        let state = self.state.clone();
        let peer_id_clone = peer_id.clone();
        let tx_clone = tx.clone();

        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(p2p_msg) = serde_json::from_str::<P2PMessage>(&text) {
                    match p2p_msg {
                        P2PMessage::Hello { node_id, node_type } => {
                            info!("🔗 Peer connected: {} ({})", node_id, node_type);
                            let peer = ConnectedPeer {
                                node_id: node_id.clone(),
                                node_type,
                                tx: tx_clone.clone(),
                            };
                            peers.write().await.insert(node_id, peer);
                        }
                        P2PMessage::GetState => {
                            let snapshot = {
                                let state_guard = state.read().await;
                                state_guard.get_state_snapshot().unwrap()
                            };
                            let _ = tx_clone.send(P2PMessage::StateSnapshot(snapshot)).await;
                        }
                        P2PMessage::SubmitTx(transaction) => {
                            let result = {
                                let mut bc = blockchain.write().await;
                                bc.add_transaction(transaction)
                            };
                            match result {
                                Ok(hash) => {
                                    info!("📤 TX received from peer: {}", &hash[..16]);
                                    let _ = tx_clone.send(P2PMessage::TxConfirmed { hash }).await;
                                }
                                Err(e) => {
                                    error!("Failed to add TX: {}", e);
                                }
                            }
                        }
                        P2PMessage::GetBlock { height } => {
                            let block = {
                                let state_guard = state.read().await;
                                state_guard.get_block(height).unwrap()
                            };
                            let _ = tx_clone.send(P2PMessage::BlockResponse(block)).await;
                        }
                        P2PMessage::Ping => {
                            let _ = tx_clone.send(P2PMessage::Pong).await;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Clean up
        peers.write().await.remove(&peer_id_clone);
        sender_task.abort();
        info!("🔌 Peer disconnected: {}", peer_id_clone);
    }

    pub fn subscribe_blocks(&self) -> broadcast::Receiver<Block> {
        self.block_tx.subscribe()
    }
}

#[async_trait]
impl Network for StarNetwork {
    async fn start(&mut self) -> Result<(), BoxError> {
        let is_master = self.config.node.node_type == "master";
        
        if is_master {
            info!("Starting P2P server for master node...");
        } else {
            let master_url = &self.config.network.star.master_url;
            if !master_url.is_empty() {
                info!("Connecting to master: {}", master_url);
            }
        }
        
        Ok(())
    }

    async fn broadcast_block(&self, block: &Block) -> Result<(), BoxError> {
        let msg = P2PMessage::NewBlock(block.clone());
        
        let peers = self.peers.read().await;
        for (_, peer) in peers.iter() {
            let _ = peer.tx.send(msg.clone()).await;
        }
        drop(peers);
        
        let browsers = self.browsers.read().await;
        for (_, tx) in browsers.iter() {
            let _ = tx.send(msg.clone()).await;
        }
        drop(browsers);
        
        let _ = self.block_tx.send(block.clone());
        
        Ok(())
    }

    fn peer_count(&self) -> usize {
        self.peers.try_read().map(|p| p.len()).unwrap_or(0)
    }

    fn browser_count(&self) -> usize {
        self.browsers.try_read().map(|b| b.len()).unwrap_or(0)
    }
}
