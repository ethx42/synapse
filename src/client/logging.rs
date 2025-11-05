use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};

/// Initialize structured logging with configurable log levels
/// 
/// Log level can be controlled via the `RUST_LOG` environment variable.
/// Examples:
/// - `RUST_LOG=info` - Info level and above
/// - `RUST_LOG=debug` - Debug level and above
/// - `RUST_LOG=synapse=debug` - Debug level for synapse crate only
/// - `RUST_LOG=warn` - Warn level and above
pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true)
        )
        .init();
}

