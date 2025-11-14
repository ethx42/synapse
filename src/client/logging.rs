use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize structured logging with default settings
///
/// Log level can be controlled via the `RUST_LOG` environment variable.
/// Examples:
/// - `RUST_LOG=info` - Info level and above
/// - `RUST_LOG=debug` - Debug level and above
/// - `RUST_LOG=synapse=debug` - Debug level for synapse crate only
/// - `RUST_LOG=warn` - Warn level and above
pub fn init_logging() {
    init_logging_with_config("info", false);
}

/// Initialize structured logging with custom configuration
///
/// # Arguments
///
/// * `log_level` - The log level to use (trace, debug, info, warn, error)
/// * `json_format` - Whether to use JSON format (true) or human-readable text (false)
///
/// # Examples
///
/// ```no_run
/// use synapse::client::init_logging_with_config;
///
/// // Text format with debug level
/// init_logging_with_config("debug", false);
///
/// // JSON format for log aggregation
/// init_logging_with_config("info", true);
/// ```
pub fn init_logging_with_config(log_level: &str, json_format: bool) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let registry = tracing_subscriber::registry().with(env_filter);

    if json_format {
        // JSON format for log aggregation systems (Datadog, Splunk, ELK, etc.)
        registry
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            )
            .init();
    } else {
        // Human-readable text format for development
        registry
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .with_file(true),
            )
            .init();
    }
}
