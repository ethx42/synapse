//! Synapse - Bare-metal application latency diagnostic tool
//!
//! This library provides functionality for measuring application-to-application latency
//! between client and server applications, including the full application stack
//! (network transmission, kernel processing, and application overhead).

pub mod client;
pub mod protocol;
pub mod server;
