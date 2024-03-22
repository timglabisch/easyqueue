pub mod raft;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use ::raft::{Config, Raft};
use ::raft::storage::MemStorage;
use async_trait::async_trait;
use bincode::{deserialize, serialize};
use tokio::runtime::Runtime;
use clap::Parser;
use log::info;
use serde::{Deserialize, Serialize};
use slog::{Logger, slog_o};
use tokio::task::JoinHandle;
use crate::raft::EbRaft;
use crate::raft::error::RaftResult;
use crate::raft::storage::Store;
use slog::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Options {
    #[arg(long)]
    raft_addr: String,
    #[arg(long)]
    peer_addr: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    Insert { key: u64, value: String },
}

#[derive(Clone)]
struct HashStore(Arc<RwLock<HashMap<u64, String>>>);


impl HashStore {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[async_trait]
impl Store for HashStore {
    async fn apply(&mut self, message: &[u8]) -> RaftResult<Vec<u8>> {
        let message: Message = deserialize(message).unwrap();
        let message: Vec<u8> = match message {
            Message::Insert { key, value } => {
                let mut db = self.0.write().unwrap();
                db.insert(key, value.clone());
                log::info!("inserted: ({}, {})", key, value);
                serialize(&value).unwrap()
            }
        };
        Ok(message)
    }

    async fn snapshot(&self) -> RaftResult<Vec<u8>> {
        Ok(serialize(&self.0.read().unwrap().clone())?)
    }

    async fn restore(&mut self, snapshot: &[u8]) -> RaftResult<()> {
        let new: HashMap<u64, String> = deserialize(snapshot).unwrap();
        let mut db = self.0.write().unwrap();
        let _ = std::mem::replace(&mut *db, new);
        Ok(())
    }
}

#[tokio::main]
async fn main() {

    let args = Options::parse();

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(drain, slog_o!("version" => env!("CARGO_PKG_VERSION")));


    let store = HashStore::new();

    let options = Options {
        raft_addr: args.raft_addr,
        peer_addr: args.peer_addr,
    };

    let raft = crate::raft::raft::Raft::new(options.raft_addr, store.clone(), logger);
    let mailbox = Arc::new(raft.mailbox());
    let (raft_handle, mailbox) = match options.peer_addr {
        Some(addr) => {
            info!("running in follower mode");
            let handle = tokio::spawn(raft.join(addr));
            (handle, mailbox)
        }
        None => {
            info!("running in leader mode");
            let handle =  tokio::spawn(raft.lead());
            (handle, mailbox)
        }
    };

    raft_handle.await;
}
