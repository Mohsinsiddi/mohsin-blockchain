pub mod star;
pub mod mesh;

use crate::chain::{Block, BoxError};
use async_trait::async_trait;

pub use star::StarNetwork;

#[async_trait]
pub trait Network: Send + Sync {
    async fn start(&mut self) -> Result<(), BoxError>;
    async fn broadcast_block(&self, block: &Block) -> Result<(), BoxError>;
    fn peer_count(&self) -> usize;
    fn browser_count(&self) -> usize;
}
