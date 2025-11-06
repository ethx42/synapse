use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use synapse::client::{
    init_logging, measurement_phase, warmup_phase, Config, NetworkSocket, Reporter, Statistics,
    UdpNetworkSocket,
};
use tracing::{error, info};

fn main() {
    // Initialize structured logging
    init_logging();

    if let Err(e) = run() {
        error!(error = %e, "Application failed");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = Config::parse();
    config
        .validate()
        .context("Failed to validate configuration")?;

    info!(server = %config.server, packets = config.packets, "Starting Synapse client");

    // Create and configure the UDP socket
    let mut socket = UdpNetworkSocket::bind("0.0.0.0:0").context("Failed to bind UDP socket")?;
    socket
        .connect(&config.server)
        .with_context(|| format!("Failed to connect to server at {}", config.server))?;
    socket
        .set_timeout(config.timeout())
        .with_context(|| format!("Failed to set socket timeout to {}ms", config.timeout_ms))?;

    println!("{}", "Synapse Application Diagnostic Tool".bold());
    println!("Server: {}\n", config.server);

    // Warmup phase
    info!(warmup_count = config.warmup, "Starting warmup phase");
    warmup_phase(&mut socket, config.warmup).context("Warmup phase failed")?;
    info!("Warmup phase completed");

    // Measurement phase
    info!(
        packet_count = config.packets,
        update_interval = config.update,
        "Starting measurement phase"
    );
    let result = measurement_phase(&mut socket, config.packets, config.update)
        .context("Measurement phase failed")?;
    info!(
        packets_received = result.latencies.len(),
        packets_lost = result.lost_packets,
        elapsed_secs = result.elapsed.as_secs_f64(),
        "Measurement phase completed"
    );

    // Analysis and reporting
    info!("Calculating statistics");
    let stats = Statistics::new(&result.latencies).with_context(|| {
        format!(
            "Failed to calculate statistics from {} latency measurements",
            result.latencies.len()
        )
    })?;
    let reporter = Reporter;

    reporter
        .print_results(
            &stats,
            result.lost_packets,
            result.total_packets,
            result.elapsed,
            &result.latencies,
        )
        .context("Failed to print results")?;

    info!("Results reported successfully");
    Ok(())
}
