//! Server configuration module
//!
//! Provides CLI argument parsing and validation for the Synapse server.

use clap::Parser;
use tracing::debug;

#[derive(Parser, Debug, Clone)]
#[command(name = "synapse-server")]
#[command(about = "High-performance TCP echo server for application diagnostics")]
pub struct ServerConfig {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Bind port
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    /// Monitor update interval in milliseconds
    #[arg(long, default_value_t = 100)]
    pub update_interval: u64,

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

impl ServerConfig {
    /// Returns the full bind address as a string (bind:port)
    pub fn address(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }

    /// Validates the configuration values
    pub fn validate(&self) -> Result<(), String> {
        debug!("Validating server configuration");

        if self.port == 0 {
            return Err("port must be > 0".into());
        }

        if self.update_interval == 0 {
            return Err("update_interval must be > 0".into());
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(format!(
                "log_level must be one of: {}",
                valid_levels.join(", ")
            ));
        }

        debug!("Server configuration validated successfully");
        Ok(())
    }

    /// Returns true if JSON format logging is enabled
    pub fn is_json_format(&self) -> bool {
        self.log_format.to_lowercase() == "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            update_interval: 100,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert_eq!(config.address(), "0.0.0.0:8080");
        assert!(!config.is_json_format());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_config() {
        let config = ServerConfig {
            bind: "127.0.0.1".to_string(),
            port: 9000,
            update_interval: 50,
            quiet: true,
            log_level: "debug".to_string(),
            log_format: "json".to_string(),
        };

        assert_eq!(config.address(), "127.0.0.1:9000");
        assert!(config.is_json_format());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let config = ServerConfig {
            bind: "0.0.0.0".to_string(),
            port: 0,
            update_interval: 100,
            quiet: false,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_log_level() {
        let config = ServerConfig {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            update_interval: 100,
            quiet: false,
            log_level: "invalid".to_string(),
            log_format: "text".to_string(),
        };

        assert!(config.validate().is_err());
    }
}
