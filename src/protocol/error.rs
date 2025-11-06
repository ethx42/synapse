use thiserror::Error;

/// Protocol-level errors for packet encoding/decoding
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid packet size: expected {expected}, got {actual}")]
    InvalidPacketSize { expected: usize, actual: usize },
}

pub type Result<T> = std::result::Result<T, ProtocolError>;
