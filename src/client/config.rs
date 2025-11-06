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
    #[arg(long, default_value_t = 100000)]
    pub warmup: usize,

    /// Dashboard update interval (packets)
    #[arg(long, default_value_t = 100)]
    pub update: usize,

    /// Socket read timeout in milliseconds
    #[arg(long, default_value_t = 100)]
    pub timeout_ms: u64,

    /// Disable terminal UI (useful for Docker/systemd/non-interactive environments)
    #[arg(long)]
    pub quiet: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Log format (text or json)
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    pub log_format: String,
}

impl Config {
    /// Returns the configured timeout as a Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// Returns true if JSON format logging is enabled
    pub fn is_json_format(&self) -> bool {
        self.log_format.to_lowercase() == "json"
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

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(ClientError::Config(format!(
                "log_level must be one of: {}",
                valid_levels.join(", ")
            )));
        }

        debug!("Configuration validated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config {
            server: "127.0.0.1:8080".to_string(),
            packets: 10000,
            warmup: 100000,
            update: 100,
            timeout_ms: 100,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert_eq!(config.server, "127.0.0.1:8080");
        assert_eq!(config.timeout(), Duration::from_millis(100));
        assert!(!config.is_json_format());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_config() {
        let config = Config {
            server: "192.168.1.1:9000".to_string(),
            packets: 50000,
            warmup: 10000,
            update: 50,
            timeout_ms: 200,
            quiet: true,
            log_level: "debug".to_string(),
            log_format: "json".to_string(),
        };

        assert_eq!(config.server, "192.168.1.1:9000");
        assert_eq!(config.timeout(), Duration::from_millis(200));
        assert!(config.is_json_format());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_packets() {
        let config = Config {
            server: "127.0.0.1:8080".to_string(),
            packets: 0,
            warmup: 100000,
            update: 100,
            timeout_ms: 100,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_timeout() {
        let config = Config {
            server: "127.0.0.1:8080".to_string(),
            packets: 10000,
            warmup: 100000,
            update: 100,
            timeout_ms: 0,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_log_level() {
        let config = Config {
            server: "127.0.0.1:8080".to_string(),
            packets: 10000,
            warmup: 100000,
            update: 100,
            timeout_ms: 100,
            quiet: false,
            log_level: "invalid".to_string(),
            log_format: "text".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_json_format_detection() {
        let mut config = Config {
            server: "127.0.0.1:8080".to_string(),
            packets: 10000,
            warmup: 100000,
            update: 100,
            timeout_ms: 100,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert!(!config.is_json_format());

        config.log_format = "json".to_string();
        assert!(config.is_json_format());

        config.log_format = "JSON".to_string(); // Case insensitive
        assert!(config.is_json_format());
    }
}
