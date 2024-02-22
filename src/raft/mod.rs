mod mailbox;
mod message;
mod error;
mod storage;
mod raft;
mod raft_node;
mod raft_service;

use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{info, instrument};

pub struct EbRaft;

impl EbRaft {
    pub async fn run() -> Result<JoinHandle<()>, ::anyhow::Error> {
        loop {
            Self::tick().await;
            ::tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[instrument]
    async fn tick() {
        info!("tick");
    }

}