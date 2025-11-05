use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Network I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Measurement error: {0}")]
    Measurement(String),

    #[error("Socket error: {0}")]
    Socket(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
