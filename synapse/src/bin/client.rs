use clap::Parser;
use hdrhistogram::Histogram;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "synapse-client")]
#[command(about = "Bare-metal network latency diagnostic tool", long_about = None)]
struct Args {
    /// Server address to connect to
    #[arg(long, default_value = "127.0.0.1:8080")]
    server: String,

    /// Number of packets to send during the test
    #[arg(long, default_value_t = 10000)]
    packets: usize,

    /// Number of warmup packets before the test
    #[arg(long, default_value_t = 200)]
    warmup: usize,

    /// Dashboard update interval (packets)
    #[arg(long, default_value_t = 100)]
    update: usize,

    /// Socket read timeout in milliseconds
    #[arg(long, default_value_t = 100)]
    timeout: u64,
}

fn main() {
    let args = Args::parse();

    // Create and configure the UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket");
    socket
        .connect(&args.server)
        .expect("Failed to connect to server");
    socket
        .set_read_timeout(Some(Duration::from_millis(args.timeout)))
        .expect("Failed to set read timeout");

    println!("Synapse Network Diagnostic Tool");
    println!("Server: {}", args.server);
    println!();

    // Warmup phase
    warmup_phase(&socket, args.warmup);

    // Measurement phase
    let (latencies, lost_packets) = measurement_phase(&socket, args.packets, args.update);

    // Analysis and reporting
    print_results(&latencies, lost_packets, args.packets);
}

fn warmup_phase(socket: &UdpSocket, warmup_count: usize) {
    println!("Warming up with {} packets...", warmup_count);
    
    let mut send_buf = [0u8; 8];
    let mut recv_buf = [0u8; 8];

    for seq in 0..warmup_count {
        send_buf[..8].copy_from_slice(&seq.to_le_bytes());
        
        // Send and receive, but discard results
        if socket.send(&send_buf).is_ok() {
            let _ = socket.recv(&mut recv_buf);
        }
    }

    println!("Warmup complete. Starting measurement...\n");
}

fn measurement_phase(socket: &UdpSocket, packet_count: usize, update_interval: usize) -> (Vec<u64>, usize) {
    // Pre-allocate vectors and buffers
    let mut latencies = Vec::with_capacity(packet_count);
    let mut lost_packets = 0usize;
    let mut send_buf = [0u8; 8];
    let mut recv_buf = [0u8; 8];

    println!("Running test with {} packets...", packet_count);

    for i in 0..packet_count {
        // Write sequence number to send buffer
        let seq = i as u64;
        send_buf[..8].copy_from_slice(&seq.to_le_bytes());

        // Timing measurement
        let t1 = Instant::now();
        
        if socket.send(&send_buf).is_err() {
            lost_packets += 1;
            continue;
        }

        match socket.recv(&mut recv_buf) {
            Ok(len) => {
                let t2 = Instant::now();
                
                // Validate received sequence number
                if len >= 8 {
                    let recv_seq = u64::from_le_bytes(recv_buf[..8].try_into().unwrap());
                    if recv_seq == seq {
                        let latency_ns = (t2 - t1).as_nanos() as u64;
                        latencies.push(latency_ns);
                    } else {
                        lost_packets += 1;
                    }
                } else {
                    lost_packets += 1;
                }
            }
            Err(_) => {
                // Timeout
                lost_packets += 1;
            }
        }

        // Live dashboard update
        if (i + 1) % update_interval == 0 || i + 1 == packet_count {
            update_dashboard(i + 1, packet_count, &latencies);
        }
    }

    // Clear the dashboard line
    println!();
    
    (latencies, lost_packets)
}

fn update_dashboard(current: usize, total: usize, latencies: &[u64]) {
    if latencies.is_empty() {
        print!("\r[Progress: {}/{}] Collecting data...", current, total);
    } else {
        let last = latencies.last().unwrap();
        let mean = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
        
        // Calculate a quick p99 estimate for live feedback
        let mut sorted = latencies.to_vec();
        sorted.sort_unstable();
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        let p99 = sorted.get(p99_idx).unwrap_or(&0);
        
        print!(
            "\r[Progress: {}/{}] Current: {:.3}ms | Mean: {:.3}ms | P99: {:.3}ms",
            current,
            total,
            *last as f64 / 1_000_000.0,
            mean / 1_000_000.0,
            *p99 as f64 / 1_000_000.0
        );
    }
    io::stdout().flush().unwrap();
}

fn print_results(latencies: &[u64], lost_packets: usize, total_packets: usize) {
    println!("\n=== Synapse Network Diagnostic Results ===\n");
    
    println!("Packets sent:     {}", total_packets);
    println!(
        "Packets lost:     {} ({:.2}%)\n",
        lost_packets,
        (lost_packets as f64 / total_packets as f64) * 100.0
    );

    if latencies.is_empty() {
        println!("No successful measurements recorded.");
        println!("\n✗ FAIL: No data to analyze");
        return;
    }

    // Build histogram with explicit bounds (100ns to 100ms)
    let mut hist = Histogram::<u64>::new_with_bounds(100, 100_000_000, 3)
        .expect("Failed to create histogram");

    for &latency in latencies {
        // Clamp values to histogram bounds
        let clamped = latency.max(100).min(100_000_000);
        hist.record(clamped).ok();
    }

    // Calculate statistics
    let mean = hist.mean();
    let min = hist.min();
    let max = hist.max();
    let p50 = hist.value_at_quantile(0.5);
    let p90 = hist.value_at_quantile(0.9);
    let p99 = hist.value_at_quantile(0.99);
    let p999 = hist.value_at_quantile(0.999);

    println!("--- Latency Statistics (RTT) ---");
    println!("Mean:             {:.1} µs", mean / 1000.0);
    println!("Minimum:          {:.1} µs", min as f64 / 1000.0);
    println!("Maximum:          {:.1} µs", max as f64 / 1000.0);
    println!("P50 (median):     {:.1} µs", p50 as f64 / 1000.0);
    println!("P90:              {:.1} µs", p90 as f64 / 1000.0);
    println!("P99:              {:.1} µs", p99 as f64 / 1000.0);
    println!("P99.9:            {:.1} µs\n", p999 as f64 / 1000.0);

    // Print histogram
    print_histogram(&hist);

    // Pass/Fail verdict
    let mean_ms = mean / 1_000_000.0;
    println!();
    if mean_ms < 1.0 {
        println!("✓ PASS: Mean latency ({:.3}ms) is below 1ms threshold", mean_ms);
    } else {
        println!("✗ FAIL: Mean latency ({:.3}ms) exceeds 1ms threshold", mean_ms);
    }
}

fn print_histogram(hist: &Histogram<u64>) {
    println!("--- Latency Distribution ---");
    
    // Create a simple ASCII histogram
    let percentiles = [
        (0.0, "Min"),
        (0.25, "P25"),
        (0.5, "P50"),
        (0.75, "P75"),
        (0.9, "P90"),
        (0.95, "P95"),
        (0.99, "P99"),
        (0.999, "P99.9"),
        (0.9999, "P99.99"),
        (1.0, "Max"),
    ];

    for (quantile, label) in percentiles {
        let value = hist.value_at_quantile(quantile);
        let value_us = value as f64 / 1000.0;
        
        // Create a simple bar (each # represents ~20µs)
        let bar_length = (value_us / 20.0).min(50.0) as usize;
        let bar = "#".repeat(bar_length);
        
        println!("{:>7}: {:>8.1} µs {}", label, value_us, bar);
    }
}

