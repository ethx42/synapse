use crate::client::error::{ClientError, Result};
use clap::Parser;
use std::time::Duration;
use tracing::debug;

#[derive(Parser, Debug, Clone)]
#[command(name = "synapse-client")]
#[command(about = "Bare-metal application latency diagnostic tool")]
pub struct Config {
    /// Server address to connect to
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub server: String,

    /// Number of packets to send during the test
    #[arg(long, default_value_t = 10000)]
    pub packets: usize,

    /// Number of warmup packets before the test
    #[arg(long, default_value_t = 200)]
    pub warmup: usize,

    /// Dashboard update interval (packets)
    #[arg(long, default_value_t = 100)]
    pub update: usize,

    /// Socket read timeout in milliseconds
    #[arg(long, default_value_t = 100)]
    pub timeout_ms: u64,
}

impl Config {
    /// Returns the configured timeout as a Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// Validates the configuration values
    pub fn validate(&self) -> Result<()> {
        debug!("Validating configuration");
        if self.packets == 0 {
            return Err(ClientError::Config("packets must be > 0".into()));
        }
        if self.timeout_ms == 0 {
            return Err(ClientError::Config("timeout must be > 0".into()));
        }
        debug!("Configuration validated successfully");
        Ok(())
    }
}
