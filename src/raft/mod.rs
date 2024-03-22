pub mod mailbox;
pub mod message;
pub mod error;
pub mod storage;
pub mod raft;
pub mod raft_node;
pub mod raft_service;
pub mod raft_server;

use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{info, instrument};

pub struct EbRaft;

impl EbRaft {

    #[instrument]
    async fn tick() {
        info!("tick");
    }

}