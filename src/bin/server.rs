use anyhow::{Context, Result};
use clap::Parser;
use synapse::client::init_logging_with_config;
use synapse::server::{ServerConfig, ServerMonitor};
use tracing::{error, info};
use std::net::UdpSocket;

fn main() {
    // Parse CLI arguments
    let config = ServerConfig::parse();

    // Initialize structured logging with config options
    init_logging_with_config(&config.log_level, config.is_json_format());

    // Validate configuration
    if let Err(e) = config.validate() {
        error!(error = %e, "Invalid configuration");
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = run(config) {
        error!(error = %e, "Server failed");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(config: ServerConfig) -> Result<()> {
    let addr = config.address();

    // Bind the UDP socket
    let socket = UdpSocket::bind(&addr)
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!(
        address = %addr,
        update_interval_ms = config.update_interval,
        quiet_mode = config.quiet,
        "Synapse server listening"
    );

    // Initialize server monitor with configured update interval
    let monitor = ServerMonitor::new(config.update_interval);
    let counters = monitor.counters();

    // Start background display thread only if not in quiet mode
    if !config.quiet {
        monitor.start_display();
    } else {
        info!("Running in quiet mode (terminal UI disabled)");
    }

    // Pre-allocate a single receive buffer outside the loop (64 bytes is sufficient)
    let mut buf = [0u8; 64];

    info!("Ready to echo packets...");

    // Infinite loop: receive and immediately echo back
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                // Increment received counter (atomic, lock-free, minimal overhead)
                counters.increment_received();

                // Immediately send back the exact same payload
                match socket.send_to(&buf[..len], src) {
                    Ok(_) => {
                        // Increment sent counter (atomic, lock-free, minimal overhead)
                        counters.increment_sent();
                    }
                    Err(e) => {
                        counters.increment_error();
                        error!(error = %e, peer = %src, "Failed to send packet");
                    }
                }
            }
            Err(e) => {
                counters.increment_error();
                error!(error = %e, "Failed to receive packet");
            }
        }
    }
}
