//! Protocol module for Synapse

pub mod error;
pub mod message;

pub use error::{ProtocolError, Result as ProtocolResult};
pub use message::{Packet, SequenceNumber, PACKET_SIZE};
