use anyhow::Result;
use clap::Parser;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use synapse::client::init_logging_with_config;
use synapse::protocol::PACKET_SIZE;
use synapse::server::{ServerConfig, ServerMonitor};
use tracing::{debug, error, info};

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

    // Bind the TCP listener
    let listener = TcpListener::bind(&addr).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            anyhow::anyhow!(
                "Failed to bind to {}: Address already in use. Try a different port or ensure no other process is using it.",
                addr
            )
        } else {
            anyhow::Error::new(e).context(format!("Failed to bind to {}", addr))
        }
    })?;

    info!(
        address = %addr,
        update_interval_ms = config.update_interval,
        quiet_mode = config.quiet,
        "Synapse TCP server listening"
    );

    // Initialize server monitor with configured update interval
    let monitor = ServerMonitor::new(config.update_interval);
    let counters = Arc::new(monitor.counters());

    // Start background display thread only if not in quiet mode
    if !config.quiet {
        monitor.start_display();
    } else {
        info!("Running in quiet mode (terminal UI disabled)");
    }

    info!("Ready to accept connections and echo packets...");

    // Accept connections and handle each in a separate thread
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let peer_addr = stream.peer_addr().ok();
                info!(peer = ?peer_addr, "New client connected");

                let counters = Arc::clone(&counters);

                // Spawn a thread to handle this client
                std::thread::spawn(move || {
                    let mut buf = [0u8; PACKET_SIZE];

                    loop {
                        // TCP is stream-based, so we must use read_exact to read exactly PACKET_SIZE bytes
                        match stream.read_exact(&mut buf) {
                            Ok(_) => {
                                counters.increment_received();

                                // Echo back the exact same payload
                                match stream.write_all(&buf) {
                                    Ok(_) => {
                                        counters.increment_sent();
                                    }
                                    Err(e) => {
                                        counters.increment_error();
                                        error!(error = %e, peer = ?peer_addr, "Failed to send packet");
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                // Check if it's a connection closed error
                                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                    debug!(peer = ?peer_addr, "Client disconnected");
                                } else {
                                    counters.increment_error();
                                    error!(error = %e, peer = ?peer_addr, "Failed to receive packet");
                                }
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                counters.increment_error();
                error!(error = %e, "Failed to accept connection");
            }
        }
    }

    Ok(())
}
