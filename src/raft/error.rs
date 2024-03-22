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

impl From<prost::DecodeError> for RaftError {
    fn from(e: prost::DecodeError) -> Self {
        Self::Other(Box::new(e))
    }
}

impl From<prost::EncodeError> for RaftError {
    fn from(e: prost::EncodeError) -> Self {
        Self::Other(Box::new(e))
    }
}

impl From<tokio::io::Error> for RaftError {
    fn from(e: tokio::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<bincode::Error> for RaftError {
    fn from(e: bincode::Error) -> Self {
        Self::Other(e)
    }
}

impl From<std::string::FromUtf8Error> for RaftError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Other(Box::new(e))
    }
}