use anyhow::{Context, Result};
use synapse::client::init_logging;
use synapse::server::ServerMonitor;
use tracing::{error, info};
use std::net::UdpSocket;

fn main() {
    // Initialize structured logging
    init_logging();

    if let Err(e) = run() {
        error!(error = %e, "Server failed");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let addr = "0.0.0.0:8080";

    // Bind the UDP socket
    let socket = UdpSocket::bind(addr)
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!(address = addr, "Synapse server listening");

    // Initialize server monitor (updates every 100ms for smooth display)
    let monitor = ServerMonitor::new(100);
    let counters = monitor.counters();

    // Start background display thread (non-blocking)
    monitor.start_display();

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
