use thiserror::Error;

pub type RaftResult<T> = std::result::Result<T, RaftError>;

#[derive(Debug, Error)]
pub enum RaftError {
    #[error("raft error: `{0}`")]
    RaftError(#[from] raft::Error),
    #[error("Error joining the cluster")]
    JoinError,
    #[error("gprc error: `{0}`")]
    Grpc(#[from] tonic::transport::Error),
    #[error("error calling remote procedure: `{0}`")]
    RemoteCall(#[from] tonic::Status),
    #[error("io error: {0}")]
    Io(String),
    #[error("unexpected error")]
    Other(#[source] Box<dyn std::error::Error + Sync + Send + 'static>),
    #[error("unexpected error")]
    Unknown,
}