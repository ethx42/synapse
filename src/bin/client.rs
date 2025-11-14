use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use synapse::client::{
    init_logging_with_config, measurement_phase, warmup_phase, Config, NetworkSocket, Reporter,
    Statistics, TcpNetworkSocket,
};
use tracing::{error, info};

fn main() {
    // Parse CLI arguments first
    let config = Config::parse();

    // Initialize structured logging with config options
    init_logging_with_config(&config.log_level, config.is_json_format());

    // Validate configuration
    if let Err(e) = config.validate() {
        error!(error = %e, "Invalid configuration");
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = run(config) {
        error!(error = %e, "Application failed");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(config: Config) -> Result<()> {
    info!(
        server = %config.server,
        packets = config.packets,
        quiet_mode = config.quiet,
        "Starting Synapse client"
    );

    // Create and configure the TCP socket
    let mut socket = TcpNetworkSocket::connect(&config.server)
        .with_context(|| format!("Failed to connect to server at {}", config.server))?;
    socket
        .set_timeout(config.timeout())
        .with_context(|| format!("Failed to set socket timeout to {}ms", config.timeout_ms))?;

    // Print header only if not in quiet mode
    if !config.quiet {
        println!("{}", "Synapse Application Diagnostic Tool".bold());
        println!("Server: {}\n", config.server);
    }

    // Warmup phase
    info!(warmup_count = config.warmup, "Starting warmup phase");
    warmup_phase(&mut socket, config.warmup, config.quiet).context("Warmup phase failed")?;
    info!("Warmup phase completed");

    // Measurement phase
    info!(
        packet_count = config.packets,
        update_interval = config.update,
        "Starting measurement phase"
    );
    let result = measurement_phase(&mut socket, config.packets, config.update, config.quiet)
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
