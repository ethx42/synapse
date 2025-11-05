use crate::client::constants::PASS_THRESHOLD_MS;
use crate::client::error::Result;
use crate::client::statistics::Statistics;
use colored::*;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Reporter for printing measurement results
pub struct Reporter;

impl Reporter {
    /// Print the complete results summary
    pub fn print_results(
        &self,
        stats: &Statistics,
        lost_packets: usize,
        total_packets: usize,
        elapsed: Duration,
        latencies: &[u64],
    ) -> Result<()> {
        debug!(
            packets_received = stats.count(),
            packets_lost = lost_packets,
            total_packets = total_packets,
            "Printing measurement results"
        );
        if stats.count() == 0 {
            warn!("No successful measurements recorded");
            println!("{}\n", "No successful measurements recorded.".red());
            println!("{}", "✗ FAIL: No data to analyze".red().bold());
            return Ok(());
        }

        let mean = stats.mean();
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

        println!(
            "Packets:  {} sent, {} lost ({:.2}%)",
            total_packets, lost_packets, loss_pct
        );
        println!("          └─ Packet loss should be 0% for reliable measurements");
        println!();
        println!("Duration: {:.2}s", elapsed_secs);
        println!(
            "          └─ Test completed at {:.1}k packets/second",
            throughput / 1000.0
        );
        println!();

        // Statistics with explanatory labels
        println!("Latency Statistics (round-trip time):");
        println!("  Mean:      {:>8.1} µs  ← Average latency", mean_us);
        println!(
            "  Min:       {:>8.1} µs  ← Fastest packet",
            stats.min() as f64 / 1000.0
        );
        println!(
            "  Max:       {:>8.1} µs  ← Slowest packet",
            stats.max() as f64 / 1000.0
        );
        println!(
            "  P50:       {:>8.1} µs  ← 50% of packets are faster than this (median)",
            stats.percentile(0.5) as f64 / 1000.0
        );
        println!(
            "  P90:       {:>8.1} µs  ← 90% of packets are faster than this",
            stats.percentile(0.9) as f64 / 1000.0
        );
        println!(
            "  P99:       {:>8.1} µs  ← 99% of packets are faster than this",
            stats.percentile(0.99) as f64 / 1000.0
        );
        println!(
            "  P99.9:     {:>8.1} µs  ← 99.9% of packets are faster than this",
            stats.percentile(0.999) as f64 / 1000.0
        );

        // Warn if values were clamped
        if stats.clamped_count() > 0 {
            println!();
            println!(
                "  ⚠ Note: {} measurement(s) exceeded histogram bounds and were clamped",
                stats.clamped_count()
            );
        }
        println!();

        // Bucket distribution (pass latencies for accurate counting)
        self.print_bucket_distribution(latencies, total_packets)?;
        println!();

        // Pass/Fail verdict with color
        let verdict = if mean_ms < PASS_THRESHOLD_MS {
            format!(
                "✓ PASS: Mean latency ({:.3}ms) is below {}ms threshold",
                mean_ms, PASS_THRESHOLD_MS
            )
            .green()
            .bold()
        } else {
            format!(
                "✗ FAIL: Mean latency ({:.3}ms) exceeds {}ms threshold",
                mean_ms, PASS_THRESHOLD_MS
            )
            .red()
            .bold()
        };

        println!("{}", verdict);

        let passed = mean_ms < PASS_THRESHOLD_MS;
        info!(
            mean_latency_ms = mean_ms,
            passed = passed,
            "Results reported"
        );

        Ok(())
    }

    /// Print bucket distribution of latencies
    pub fn print_bucket_distribution(&self, latencies: &[u64], total_packets: usize) -> Result<()> {
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

            println!(
                "  {:>12}:  {:30} {} ({:7} packets)",
                label_colored,
                bar,
                pct_str,
                Self::format_count(count)
            );
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

            println!(
                "  {:>12}:  {:30} {} ({:7} packets) ← MAX: {:.1}ms",
                ">10 ms".red().bold(),
                "▌".repeat(1),
                pct_str,
                Self::format_count(outliers),
                max_ms
            );
        }

        Ok(())
    }

    fn format_count(count: usize) -> String {
        if count >= 1000 {
            format!("{:>6}k", count / 1000)
        } else {
            format!("{:>7}", count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporter_print_results_empty() -> Result<()> {
        let reporter = Reporter;
        let stats = Statistics::new(&[])?;

        // Should handle empty latencies gracefully
        reporter.print_results(&stats, 0, 10, Duration::from_secs(1), &[])?;
        Ok(())
    }

    #[test]
    fn test_reporter_print_results_with_data() -> Result<()> {
        let reporter = Reporter;
        let latencies = vec![1000, 2000, 3000, 4000, 5000];
        let stats = Statistics::new(&latencies)?;

        reporter.print_results(&stats, 0, 5, Duration::from_secs(1), &latencies)?;
        Ok(())
    }

    #[test]
    fn test_reporter_print_bucket_distribution() -> Result<()> {
        let reporter = Reporter;
        let latencies = vec![
            10000,  // 10 µs
            20000,  // 20 µs
            50000,  // 50 µs
            100000, // 100 µs
            500000, // 500 µs
        ];

        reporter.print_bucket_distribution(&latencies, 5)?;
        Ok(())
    }

    #[test]
    fn test_reporter_format_count() {
        assert_eq!(Reporter::format_count(100), "    100");
        assert_eq!(Reporter::format_count(1000), "     1k");
        assert_eq!(Reporter::format_count(5000), "     5k");
    }
}
