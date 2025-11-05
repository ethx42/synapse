use synapse::client::{Config, NetworkSocket, UdpNetworkSocket, warmup_phase, measurement_phase, Statistics, Reporter};
use synapse::client::Result;
use clap::Parser;
use colored::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = Config::parse();
    config.validate()?;

    // Create and configure the UDP socket
    let mut socket = UdpNetworkSocket::bind("0.0.0.0:0")?;
    socket.connect(&config.server)?;
    socket.set_timeout(config.timeout())?;

    println!("{}", "Synapse Application Diagnostic Tool".bold());
    println!("Server: {}\n", config.server);

    // Warmup phase
    warmup_phase(&mut socket, config.warmup)?;

    // Measurement phase
    let result = measurement_phase(&mut socket, config.packets, config.update)?;

    // Analysis and reporting
    let stats = Statistics::new(&result.latencies)?;
    let reporter = Reporter;
    
    reporter.print_results(&stats, result.lost_packets, result.total_packets, result.elapsed, &result.latencies)?;
    
    Ok(())
}
