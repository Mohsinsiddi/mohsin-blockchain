mod config;
mod chain;
mod mvm;
mod standards;
mod address;
mod state;
mod network;
mod api;

use crate::config::Config;
use crate::chain::Blockchain;
use crate::state::State;
use crate::network::{Network, StarNetwork};
use crate::api::start_api_server;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    
    let config_path = if args.len() > 2 && args[1] == "--config" {
        args[2].clone()
    } else {
        "config.toml".to_string()
    };

    // Load config
    let config = Config::load(&config_path)?;
    
    // Setup logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(match config.logging.level.as_str() {
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        })
        .with_target(false)
        .pretty()
        .init();

    print_banner();
    
    info!("Loading config from: {}", config_path);
    info!("Chain ID: {}", config.chain.chain_id);
    info!("Node ID: {}", config.node.id);
    info!("Node Type: {}", config.node.node_type);

    // Initialize state (RocksDB)
    let state = Arc::new(RwLock::new(State::new(&config.node.data_dir)?));
    
    // Generate or load master address
    let master_address = {
        let mut state_guard = state.write().await;
        let addr = state_guard.get_or_create_master_address()?;
        info!("Master Address: {}", addr);
        addr
    };

    // Initialize blockchain
    let blockchain = Arc::new(RwLock::new(
        Blockchain::new(config.clone(), state.clone(), master_address.clone()).await?
    ));

    // Initialize network
    let network = Arc::new(RwLock::new(
        StarNetwork::new(config.clone(), blockchain.clone(), state.clone())
    ));

    // Start network
    {
        let mut net = network.write().await;
        net.start().await?;
    }

    // Start API server
    let api_handle = tokio::spawn(start_api_server(
        config.clone(),
        blockchain.clone(),
        state.clone(),
        network.clone(),
    ));

    // If master, start block production
    if config.node.node_type == "master" {
        let bc = blockchain.clone();
        let net = network.clone();
        let block_time = config.block.block_time;
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(block_time)).await;
                
                let mut blockchain = bc.write().await;
                match blockchain.produce_block().await {
                    Ok(block) => {
                        info!("📦 Block #{} produced | {} txs | hash: {}",
                            block.height,
                            block.transactions.len(),
                            &block.hash[..16]
                        );
                        
                        // Broadcast to connected nodes
                        let network = net.read().await;
                        if let Err(e) = network.broadcast_block(&block).await {
                            tracing::error!("Failed to broadcast block: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to produce block: {}", e);
                    }
                }
            }
        });
    }

    // Print status
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🟢 Node is LIVE");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("P2P:  ws://{}:{}", config.network.host, config.network.p2p_port);
    info!("WS:   ws://{}:{}", config.network.host, config.network.ws_port);
    info!("API:  http://{}:{}", config.network.host, config.network.api_port);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Wait for API server
    api_handle.await??;

    Ok(())
}

fn print_banner() {
    println!(r#"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ███╗   ███╗ ██╗   ██╗ ███╗   ███╗                          ║
║   ████╗ ████║ ██║   ██║ ████╗ ████║                          ║
║   ██╔████╔██║ ██║   ██║ ██╔████╔██║                          ║
║   ██║╚██╔╝██║ ╚██╗ ██╔╝ ██║╚██╔╝██║                          ║
║   ██║ ╚═╝ ██║  ╚████╔╝  ██║ ╚═╝ ██║                          ║
║   ╚═╝     ╚═╝   ╚═══╝   ╚═╝     ╚═╝                          ║
║                                                               ║
║   MOHSIN VIRTUAL MACHINE                                      ║
║   A Simple Blockchain with MVM-20 Token Standard              ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
"#);
}
