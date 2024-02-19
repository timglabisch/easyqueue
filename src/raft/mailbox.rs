use std::time::Duration;
use raft::eraftpb::{ConfChange, ConfChangeType};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use crate::raft::error::{RaftError, RaftResult};
use crate::raft::message::{Message, RaftResponse};

#[derive(Clone)]
pub struct Mailbox(mpsc::Sender<Message>);

impl Mailbox {
    /// sends a proposal message to commit to the node. This fails if the current node is not the
    /// leader
    pub async fn send(&self, message: Vec<u8>) -> RaftResult<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        let proposal = Message::Propose {
            proposal: message,
            chan: tx,
        };
        let sender = self.0.clone();
        // TODO make timeout duration a variable
        match sender.send(proposal).await {
            Ok(_) => match timeout(Duration::from_secs(2), rx).await {
                Ok(Ok(RaftResponse::Response { data })) => Ok(data),
                _ => Err(RaftError::Unknown),
            },
            _ => Err(RaftError::Unknown),
        }
    }

    pub async fn leave(&self) -> RaftResult<()> {
        let mut change = ConfChange::default();
        // set node id to 0, the node will set it to self when it receives it.
        change.set_node_id(0);
        change.set_change_type(ConfChangeType::RemoveNode);
        let sender = self.0.clone();
        let (chan, rx) = oneshot::channel();
        match sender.send(Message::ConfigChange { change, chan }).await {
            Ok(_) => match rx.await {
                Ok(RaftResponse::Ok) => Ok(()),
                _ => Err(RaftError::Unknown),
            },
            _ => Err(RaftError::Unknown),
        }
    }
}