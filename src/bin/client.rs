use clap::Parser;
use colored::*;
use hdrhistogram::Histogram;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

// OSI Layer visualization state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum PacketPosition {
    ClientL7,
    ClientL4,
    ClientL3,
    ClientL2,
    ClientL1,
    Network,
    ServerL1,
    ServerL2,
    ServerL3,
    ServerL4,
    ServerL7,
    ReturnServerL7,
    ReturnServerL4,
    ReturnServerL3,
    ReturnServerL2,
    ReturnServerL1,
    ReturnNetwork,
    ReturnClientL1,
    ReturnClientL2,
    ReturnClientL3,
    ReturnClientL4,
    ReturnClientL7,
}

impl PacketPosition {
    fn next(self) -> Self {
        match self {
            PacketPosition::ClientL7 => PacketPosition::ClientL4,
            PacketPosition::ClientL4 => PacketPosition::ClientL3,
            PacketPosition::ClientL3 => PacketPosition::ClientL2,
            PacketPosition::ClientL2 => PacketPosition::ClientL1,
            PacketPosition::ClientL1 => PacketPosition::Network,
            PacketPosition::Network => PacketPosition::ServerL1,
            PacketPosition::ServerL1 => PacketPosition::ServerL2,
            PacketPosition::ServerL2 => PacketPosition::ServerL3,
            PacketPosition::ServerL3 => PacketPosition::ServerL4,
            PacketPosition::ServerL4 => PacketPosition::ServerL7,
            PacketPosition::ServerL7 => PacketPosition::ReturnServerL7,
            PacketPosition::ReturnServerL7 => PacketPosition::ReturnServerL4,
            PacketPosition::ReturnServerL4 => PacketPosition::ReturnServerL3,
            PacketPosition::ReturnServerL3 => PacketPosition::ReturnServerL2,
            PacketPosition::ReturnServerL2 => PacketPosition::ReturnServerL1,
            PacketPosition::ReturnServerL1 => PacketPosition::ReturnNetwork,
            PacketPosition::ReturnNetwork => PacketPosition::ReturnClientL1,
            PacketPosition::ReturnClientL1 => PacketPosition::ReturnClientL2,
            PacketPosition::ReturnClientL2 => PacketPosition::ReturnClientL3,
            PacketPosition::ReturnClientL3 => PacketPosition::ReturnClientL4,
            PacketPosition::ReturnClientL4 => PacketPosition::ReturnClientL7,
            PacketPosition::ReturnClientL7 => PacketPosition::ClientL7,
        }
    }
}

struct OsiState {
    position: PacketPosition,
}

impl OsiState {
    fn new() -> Self {
        Self {
            position: PacketPosition::ClientL7,
        }
    }

    fn advance(&mut self) {
        self.position = self.position.next();
    }
}

fn render_layer(label: &str, detail: &str, is_active: bool, layer_color: (u8, u8, u8)) -> String {
    let (r, g, b) = layer_color;
    let text = format!("{}: {}", label, detail);
    
    if is_active {
        // Bright background color when active with white text
        format!("{}", 
            format!(" {:<20} ", text)
                .on_truecolor(r, g, b)
                .truecolor(255, 255, 255)
                .bold()
        )
    } else {
        // Dim background color when inactive (reduce brightness by ~70%)
        let dim_r = (r as f32 * 0.3) as u8;
        let dim_g = (g as f32 * 0.3) as u8;
        let dim_b = (b as f32 * 0.3) as u8;
        format!("{}", 
            format!(" {:<20} ", text)
                .on_truecolor(dim_r, dim_g, dim_b)
                .truecolor(100, 100, 100)
        )
    }
}

fn render_osi_stack(osi_state: &OsiState) -> String {
    let pos = osi_state.position;
    
    // Check which layers are active on each side
    let client_l7_active = matches!(pos, PacketPosition::ClientL7 | PacketPosition::ReturnClientL7);
    let client_l4_active = matches!(pos, PacketPosition::ClientL4 | PacketPosition::ReturnClientL4);
    let client_l3_active = matches!(pos, PacketPosition::ClientL3 | PacketPosition::ReturnClientL3);
    let client_l2_active = matches!(pos, PacketPosition::ClientL2 | PacketPosition::ReturnClientL2);
    let client_l1_active = matches!(pos, PacketPosition::ClientL1 | PacketPosition::ReturnClientL1);
    
    let server_l7_active = matches!(pos, PacketPosition::ServerL7 | PacketPosition::ReturnServerL7);
    let server_l4_active = matches!(pos, PacketPosition::ServerL4 | PacketPosition::ReturnServerL4);
    let server_l3_active = matches!(pos, PacketPosition::ServerL3 | PacketPosition::ReturnServerL3);
    let server_l2_active = matches!(pos, PacketPosition::ServerL2 | PacketPosition::ReturnServerL2);
    let server_l1_active = matches!(pos, PacketPosition::ServerL1 | PacketPosition::ReturnServerL1);
    
    // Layer colors (RGB): Blue, Green, Yellow, Orange, Red
    let l7_color = (74, 144, 226);   // Blue
    let l4_color = (72, 187, 120);   // Green
    let l3_color = (236, 201, 75);   // Yellow
    let l2_color = (237, 137, 54);   // Orange
    let l1_color = (245, 101, 101);  // Red
    
    let mut lines = Vec::new();
    
    // Header - centered above stacks
    lines.push(format!("                 {}                  {}", 
        "CLIENT".bold(),
        "SERVER".bold()
    ));
    
    // Layer 7
    lines.push(format!("         {}  {}", 
        render_layer("L7", "APPLICATION", client_l7_active, l7_color),
        render_layer("L7", "APPLICATION", server_l7_active, l7_color)
    ));
    
    // Layer 4
    lines.push(format!("{}  {}", 
        render_layer("L4", "TRANSPORT", client_l4_active, l4_color),
        render_layer("L4", "TRANSPORT", server_l4_active, l4_color)
    ));
    
    // Layer 3
    lines.push(format!("{}  {}", 
        render_layer("L3", "NETWORK", client_l3_active, l3_color),
        render_layer("L3", "NETWORK", server_l3_active, l3_color)
    ));
    
    // Layer 2
    lines.push(format!("{}  {}", 
        render_layer("L2", "DATA LINK", client_l2_active, l2_color),
        render_layer("L2", "DATA LINK", server_l2_active, l2_color)
    ));
    
    // Layer 1
    lines.push(format!("{}  {}", 
        render_layer("L1", "PHYSICAL", client_l1_active, l1_color),
        render_layer("L1", "PHYSICAL", server_l1_active, l1_color)
    ));
    
    // Network connection
    let network_arrow = if matches!(pos, PacketPosition::Network) {
        format!("                    {}        ", "──────────▶".bright_cyan().bold())
    } else if matches!(pos, PacketPosition::ReturnNetwork) {
        format!("                    {}        ", "◀──────────".bright_cyan().bold())
    } else {
        "                      ".to_string()
    };
    lines.push(network_arrow);
    
    lines.join("\n")
}

#[derive(Parser, Debug)]
#[command(name = "synapse-client")]
#[command(about = "Bare-metal application latency diagnostic tool", long_about = None)]
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

    println!("{}", "Synapse Application Diagnostic Tool".bold());
    println!("Server: {}\n", args.server);

    // Warmup phase
    warmup_phase(&socket, args.warmup);

    // Measurement phase
    let (latencies, lost_packets, elapsed) = measurement_phase(&socket, args.packets, args.update);

    // Analysis and reporting
    print_results(&latencies, lost_packets, args.packets, elapsed);
}

fn warmup_phase(socket: &UdpSocket, warmup_count: usize) {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut spinner_idx = 0;
    
    let mut send_buf = [0u8; 8];
    let mut recv_buf = [0u8; 8];

    for seq in 0..warmup_count {
        send_buf[..8].copy_from_slice(&seq.to_le_bytes());
        
        // Send and receive, but discard results
        if socket.send(&send_buf).is_ok() {
            let _ = socket.recv(&mut recv_buf);
        }
        
        // Update spinner every 10 packets for smooth animation
        if seq % 10 == 0 {
            print!("\rWarming up {} ({}/{})", 
                spinner_chars[spinner_idx], 
                seq + 1, 
                warmup_count
            );
            io::stdout().flush().unwrap();
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();
        }
    }
    
    println!("\rWarming up ✓ ({}/{})", warmup_count, warmup_count);
    println!();
}

fn measurement_phase(socket: &UdpSocket, packet_count: usize, update_interval: usize) -> (Vec<u64>, usize, Duration) {
    // Pre-allocate vectors and buffers
    let mut latencies = Vec::with_capacity(packet_count);
    let mut lost_packets = 0usize;
    let mut send_buf = [0u8; 8];
    let mut recv_buf = [0u8; 8];
    
    let start_time = Instant::now();
    let mut last_update = Instant::now();
    
    // OSI visualization state
    let mut osi_state = OsiState::new();
    const SAMPLE_RATE: usize = 100; // Animate every 100th packet

    // Create progress bar
    let pb = ProgressBar::new(packet_count as u64);
    pb.set_style(
        ProgressStyle::with_template("{msg}\n{bar:40.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]")
            .unwrap()
            .progress_chars("█░")
    );
    
    // Enable indicatif's steady tick for smooth updates
    pb.enable_steady_tick(Duration::from_millis(100));

    for i in 0..packet_count {
        // Write sequence number to send buffer
        let seq = i as u64;
        send_buf[..8].copy_from_slice(&seq.to_le_bytes());

        // Timing measurement
        let t1 = Instant::now();
        
        if socket.send(&send_buf).is_err() {
            lost_packets += 1;
            pb.inc(1);
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

        // Update progress bar and live stats
        pb.inc(1);
        
        // Update live stats less frequently to avoid clutter (every update_interval packets or every 500ms)
        if (i + 1) % update_interval == 0 || last_update.elapsed().as_millis() > 500 {
            if !latencies.is_empty() {
                update_live_stats(&pb, &latencies, start_time, &osi_state);
            }
            last_update = Instant::now();
        }
        
        // Advance OSI animation on sampled packets
        if (i + 1) % SAMPLE_RATE == 0 {
            osi_state.advance();
        }
    }

    // Update stats one final time before finishing
    if !latencies.is_empty() {
        update_live_stats(&pb, &latencies, start_time, &osi_state);
    }
    
    // Finish progress bar but keep it visible
    pb.finish();
    println!(); // Add blank line for separation
    
    let elapsed = start_time.elapsed();
    (latencies, lost_packets, elapsed)
}

fn update_live_stats(
    pb: &ProgressBar,
    latencies: &[u64],
    start_time: Instant,
    osi_state: &OsiState,
) {
    let last = latencies.last().unwrap();
    let mean = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
    
    // Calculate a quick p99 estimate for live feedback
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;
    let p99 = sorted.get(p99_idx).unwrap_or(&0);
    
    // Calculate packet rate
    let elapsed = start_time.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        latencies.len() as f64 / elapsed
    } else {
        0.0
    };
    
    // Color code latency
    let last_ms = *last as f64 / 1_000_000.0;
    let mean_ms = mean / 1_000_000.0;
    let p99_ms = *p99 as f64 / 1_000_000.0;
    
    let last_str = format!("{:.3}", last_ms);
    let mean_str = format!("{:.3}", mean_ms);
    
    let last_color = if last_ms < 0.5 {
        last_str.green()
    } else if last_ms < 1.0 {
        last_str.yellow()
    } else {
        last_str.red()
    };
    
    let mean_color = if mean_ms < 1.0 {
        mean_str.green()
    } else {
        mean_str.red()
    };
    
    // Render OSI visualization
    let osi_viz = render_osi_stack(osi_state);
    let osi_lines: Vec<&str> = osi_viz.lines().collect();
    
    // Build combined display with metrics on left, OSI on right
    let metrics_lines = vec![
        format!("→ {}ms", last_color),
        format!("Mean: {}ms", mean_color),
        format!("P99: {:.3}ms", p99_ms),
        format!("Rate: {:.1}k pkt/s", rate / 1000.0),
    ];
    
    let mut combined = Vec::new();
    
    // Combine metrics and OSI lines side by side
    let max_lines = metrics_lines.len().max(osi_lines.len());
    for i in 0..max_lines {
        let metric_part = if i < metrics_lines.len() {
            format!("{:<25}", metrics_lines[i])
        } else {
            " ".repeat(25)
        };
        
        let osi_part = if i < osi_lines.len() {
            osi_lines[i].to_string()
        } else {
            String::new()
        };
        
        combined.push(format!("{}{}", metric_part, osi_part));
    }
    
    // Use indicatif's message field with newlines - updates in place without creating new lines
    let msg = combined.join("\n");
    pb.set_message(msg);
}

fn print_results(latencies: &[u64], lost_packets: usize, total_packets: usize, elapsed: Duration) {
    if latencies.is_empty() {
        println!("{}\n", "No successful measurements recorded.".red());
        println!("{}", "✗ FAIL: No data to analyze".red().bold());
        return;
    }

    // Calculate real min/max BEFORE clamping (for accurate reporting)
    let real_min = latencies.iter().min().copied().unwrap_or(0);
    let real_max = latencies.iter().max().copied().unwrap_or(0);
    let mut clamped_count = 0usize;
    
    // Build histogram with explicit bounds (100ns to 100ms)
    let mut hist = Histogram::<u64>::new_with_bounds(100, 100_000_000, 3)
        .expect("Failed to create histogram");

    for &latency in latencies {
        // Clamp values to histogram bounds for percentile calculations
        let clamped = latency.max(100).min(100_000_000);
        if latency != clamped {
            clamped_count += 1;
        }
        hist.record(clamped).ok();
    }

    // Calculate statistics from histogram (for percentiles)
    let mean = hist.mean();
    let p50 = hist.value_at_quantile(0.5);
    let p90 = hist.value_at_quantile(0.9);
    let p99 = hist.value_at_quantile(0.99);
    let p999 = hist.value_at_quantile(0.999);
    
    // Use real min/max from original latencies (not clamped histogram values)
    let min = real_min;
    let max = real_max;

    // Convert to microseconds
    let mean_us = mean / 1000.0;
    let mean_ms = mean / 1_000_000.0;
    let loss_pct = (lost_packets as f64 / total_packets as f64) * 100.0;

    // Print minimalistic summary
    println!("\n{}", "┌─────────────────────────────┐".cyan());
    println!("{}", "│  Synapse Results            │".cyan());
    println!("{}", "└─────────────────────────────┘".cyan());
    println!();
    
    // Key metrics with explanatory labels
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = total_packets as f64 / elapsed_secs;
    
    println!("Packets:  {} sent, {} lost ({:.2}%)", 
        total_packets, 
        lost_packets,
        loss_pct
    );
    println!("          └─ Packet loss should be 0% for reliable measurements");
    println!();
    println!("Duration: {:.2}s", elapsed_secs);
    println!("          └─ Test completed at {:.1}k packets/second", throughput / 1000.0);
    println!();
    
    // Statistics with explanatory labels
    println!("Latency Statistics (round-trip time):");
    println!("  Mean:      {:>8.1} µs  ← Average latency", mean_us);
    println!("  Min:       {:>8.1} µs  ← Fastest packet", min as f64 / 1000.0);
    println!("  Max:       {:>8.1} µs  ← Slowest packet", max as f64 / 1000.0);
    println!("  P50:       {:>8.1} µs  ← 50% of packets are faster than this (median)", p50 as f64 / 1000.0);
    println!("  P90:       {:>8.1} µs  ← 90% of packets are faster than this", p90 as f64 / 1000.0);
    println!("  P99:       {:>8.1} µs  ← 99% of packets are faster than this", p99 as f64 / 1000.0);
    println!("  P99.9:     {:>8.1} µs  ← 99.9% of packets are faster than this", p999 as f64 / 1000.0);
    
    // Warn if values were clamped
    if clamped_count > 0 {
        println!();
        println!("  ⚠ Note: {} measurement(s) exceeded histogram bounds and were clamped", clamped_count);
    }
    println!();

    // Bucket distribution (pass latencies for accurate counting)
    print_bucket_distribution(&latencies, total_packets);
    println!();

    // Pass/Fail verdict with color
    let verdict = if mean_ms < 1.0 {
        format!("✓ PASS: Mean latency ({:.3}ms) is below 1ms threshold", mean_ms)
            .green()
            .bold()
    } else {
        format!("✗ FAIL: Mean latency ({:.3}ms) exceeds 1ms threshold", mean_ms)
            .red()
            .bold()
    };
    
    println!("{}", verdict);
}

fn print_bucket_distribution(latencies: &[u64], total_packets: usize) {
    println!("Latency Distribution (packet count by range):");
    
    // Define buckets in microseconds
    let buckets: Vec<(f64, f64, &str)> = vec![
        (0.0, 20.0, "0-20 µs"),
        (20.0, 40.0, "20-40 µs"),
        (40.0, 60.0, "40-60 µs"),
        (60.0, 80.0, "60-80 µs"),
        (80.0, 100.0, "80-100 µs"),
        (100.0, 200.0, "100-200 µs"),
        (200.0, 500.0, "200-500 µs"),
        (500.0, 1000.0, "500µs-1ms"),
        (1000.0, 10000.0, "1-10 ms"),
    ];
    
    // Count packets in each bucket
    let mut bucket_counts = vec![0usize; buckets.len()];
    let mut outliers = 0usize;
    let mut max_latency = 0u64;
    
    for &latency_ns in latencies {
        let latency_us = latency_ns as f64 / 1000.0;
        max_latency = max_latency.max(latency_ns);
        
        let mut found = false;
        for (i, &(min, max, _)) in buckets.iter().enumerate() {
            if latency_us >= min && latency_us < max {
                bucket_counts[i] += 1;
                found = true;
                break;
            }
        }
        
        if !found && latency_us >= 10000.0 {
            outliers += 1;
        }
    }
    
    // Find max count for bar scaling
    let max_count = *bucket_counts.iter().max().unwrap_or(&1);
    let bar_width = 30;
    
    // Print each bucket
    for (i, &(_, _, label)) in buckets.iter().enumerate() {
        let count = bucket_counts[i];
        if count == 0 && i > 5 {
            continue; // Skip empty buckets beyond 100µs for cleaner output
        }
        
        let percentage = (count as f64 / total_packets as f64) * 100.0;
        
        // Calculate bar length
        let bar_length = if max_count > 0 {
            ((count as f64 / max_count as f64) * bar_width as f64) as usize
        } else {
            0
        };
        
        // Use different characters for different bar lengths
        let bar = if bar_length > 0 {
            if bar_length >= bar_width {
                "█".repeat(bar_width)
            } else if bar_length >= 2 {
                "█".repeat(bar_length)
            } else {
                "▌".to_string()
            }
        } else {
            "".to_string()
        };
        
        // Color code based on performance
        let label_colored = if percentage > 50.0 {
            label.green()
        } else if percentage > 10.0 {
            label.cyan()
        } else {
            label.normal()
        };
        
        // Use more precision for small percentages
        let pct_str = if percentage < 0.1 {
            format!("{:5.3}%", percentage)
        } else if percentage < 1.0 {
            format!("{:5.2}%", percentage)
        } else {
            format!("{:5.1}%", percentage)
        };
        
        println!("  {:>12}:  {:30} {} ({:7} packets)", 
            label_colored, bar, pct_str, 
            format_count(count));
    }
    
    // Print outliers if any
    if outliers > 0 {
        let percentage = (outliers as f64 / total_packets as f64) * 100.0;
        let max_ms = max_latency as f64 / 1_000_000.0;
        
        let pct_str = if percentage < 0.1 {
            format!("{:5.3}%", percentage)
        } else if percentage < 1.0 {
            format!("{:5.2}%", percentage)
        } else {
            format!("{:5.1}%", percentage)
        };
        
        println!("  {:>12}:  {:30} {} ({:7} packets) ← MAX: {:.1}ms",
            ">10 ms".red().bold(),
            "▌".repeat(1),
            pct_str,
            format_count(outliers),
            max_ms
        );
    }
}

fn format_count(count: usize) -> String {
    if count >= 1000 {
        format!("{:>6}k", count / 1000)
    } else {
        format!("{:>7}", count)
    }
}
