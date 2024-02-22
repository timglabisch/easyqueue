use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};
use crate::raft::mailbox::Mailbox;
use crate::raft::message::Message;
use crate::raft::storage::Store;

pub struct Raft<S: Store + 'static> {
    store: S,
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    addr: String,
}

impl<S: Store + Send + Sync + 'static> Raft<S> {
    /// creates a new node with the given address and store.
    pub fn new(addr: String, store: S) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            store,
            tx,
            rx,
            addr,
        }
    }

    /// gets the node's `Mailbox`.
    pub fn mailbox(&self) -> Mailbox {
        Mailbox(self.tx.clone())
    }

    /// Create a new leader for the cluster, with id 1. There has to be exactly one node in the
    /// cluster that is initialised that way
    pub async fn lead(self) -> Result<()> {
        let addr = self.addr.clone();
        let node = RaftNode::new_leader(self.rx, self.tx.clone(), self.store, &self.logger);
        let server = RaftServer::new(self.tx, addr);
        let _server_handle = tokio::spawn(server.run());
        let node_handle = tokio::spawn(node.run());
        let _ = tokio::try_join!(node_handle);
        warn!("leaving leader node");

        Ok(())
    }

    /// Tries to join a new cluster at `addr`, getting an id from the leader, or finding it if
    /// `addr` is not the current leader of the cluster
    pub async fn join(self, addr: String) -> Result<()> {
        // 1. try to discover the leader and obtain an id from it.
        info!("attempting to join peer cluster at {}", addr);
        let mut leader_addr = addr.to_string();
        let (leader_id, node_id, peer_addrs): (u64, u64, HashMap<u64, String>) = loop {
            let mut client = RaftServiceClient::connect(format!("http://{}", leader_addr)).await?;
            let response = client
                .request_id(Request::new(RequestIdArgs {
                    addr: self.addr.clone(),
                }))
                .await?
                .into_inner();
            match response.code() {
                ResultCode::WrongLeader => {
                    info!("this is the wrong leader");
                    let (_leader_id, addr): (u64, String) = deserialize(&response.data)?;
                    leader_addr = addr;
                    info!("Wrong leader, retrying with leader at {}", leader_addr);
                    continue;
                }
                ResultCode::Ok => {
                    break deserialize(&response.data)?;
                }
                ResultCode::Error => return Err(Error::JoinError),
            }
        };

        info!("obtained ID from leader: {}", node_id);
        // 2. run server and node to prepare for joining
        let addr = self.addr.clone();
        let mut node =
            RaftNode::new_follower(self.rx, self.tx.clone(), node_id, self.store, &self.logger)?;
        for (id, peer_addr) in peer_addrs.iter() {
            node.add_peer(peer_addr, id.to_owned()).await?;
        }
        node.add_peer(&leader_addr, leader_id).await?;
        let mut client = node.peer_mut(leader_id).unwrap().clone();
        let server = RaftServer::new(self.tx, addr);
        let _server_handle = tokio::spawn(server.run());
        let node_handle = tokio::spawn(node.run());

        // 3. Join the cluster
        // TODO: handle wrong leader
        let mut change = ConfChange::default();
        change.set_node_id(node_id);
        change.set_change_type(ConfChangeType::AddNode);
        change.set_context(serialize(&self.addr)?);
        client.change_config(Request::new(change)).await?;
        let _ = tokio::try_join!(node_handle);

        Ok(())
    }
}