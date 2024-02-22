use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::raft::message::Message;
use raft::eraftpb::{ConfChange, ConfChangeType, Entry, EntryType, Message as RaftMessage};
use raft::{prelude::*, Config, RawNode};
use tonic::Request;
use tonic::transport::Channel;
use tracing::{debug, info};
use crate::raft::error::RaftResult;
use crate::raft::raft_service::raft_service_client::RaftServiceClient;
use crate::raft::storage::{LogStore, MemStorage, Store};



struct MessageSender {
    message: RaftMessage,
    client: RaftServiceClient<tonic::transport::channel::Channel>,
    client_id: u64,
    chan: mpsc::Sender<Message>,
    max_retries: usize,
    timeout: Duration,
}

impl MessageSender {
    /// attempt to send a message MessageSender::max_retries times at MessageSender::timeout
    /// inteval.
    async fn send(mut self) {
        let mut current_retry = 0usize;
        loop {
            let message_request = Request::new(self.message.clone());
            match self.client.send_message(message_request).await {
                Ok(_) => {
                    return;
                }
                Err(e) => {
                    if current_retry < self.max_retries {
                        current_retry += 1;
                        tokio::time::sleep(self.timeout).await;
                    } else {
                        debug!(
                            "error sending message after {} retries: {}",
                            self.max_retries, e
                        );
                        let _ = self
                            .chan
                            .send(Message::ReportUnreachable {
                                node_id: self.client_id,
                            })
                            .await;
                        return;
                    }
                }
            }
        }
    }
}


pub struct Peer {
    addr: String,
    client: RaftServiceClient<Channel>,
}


impl Deref for Peer {
    type Target = RaftServiceClient<Channel>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for Peer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl Peer {
    pub async fn new(addr: &str) -> RaftResult<Peer> {
        // TODO: clean up this mess
        info!("connecting to node at {}...", addr);
        let client = RaftServiceClient::connect(format!("http://{}", addr)).await?;
        let addr = addr.to_string();
        info!("connected to node.");
        Ok(Peer { addr, client })
    }
}

pub struct RaftNode<S: Store> {
    inner: RawNode<MemStorage>,
    // the peer is optional, because an id can be reserved and later populated
    pub peers: HashMap<u64, Option<Peer>>,
    pub rcv: mpsc::Receiver<Message>,
    pub snd: mpsc::Sender<Message>,
    store: S,
    should_quit: bool,
    seq: AtomicU64,
    last_snap_time: Instant,
}

impl<S: Store + 'static + Send> RaftNode<S> {
    pub fn new_leader(
        rcv: mpsc::Receiver<Message>,
        snd: mpsc::Sender<Message>,
        store: S,
        logger: &slog::Logger,
    ) -> Self {
        let config = Config {
            id: 1,
            election_tick: 10,
            // Heartbeat tick is for how long the leader needs to send
            // a heartbeat to keep alive.
            heartbeat_tick: 3,
            // Just for log
            ..Default::default()
        };

        config.validate().unwrap();

        let mut s = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];

        let mut storage = MemStorage::create();
        storage.apply_snapshot(s).unwrap();
        let mut inner = RawNode::new(&config, storage, logger).unwrap();
        let peers = HashMap::new();
        let seq = AtomicU64::new(0);
        let last_snap_time = Instant::now();

        inner.raft.become_candidate();
        inner.raft.become_leader();

        RaftNode {
            inner,
            rcv,
            peers,
            store,
            seq,
            snd,
            should_quit: false,
            last_snap_time,
        }
    }
}