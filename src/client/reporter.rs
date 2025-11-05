use crate::client::constants::PASS_THRESHOLD_MS;
use crate::client::error::Result;
use crate::client::statistics::Statistics;
use colored::*;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Reporter for printing measurement results
pub struct Reporter;

// Constants for histogram visualization
const HISTOGRAM_BAR_WIDTH: usize = 30;
const OUTLIER_THRESHOLD_US: f64 = 10_000.0;
const EMPTY_BUCKET_SKIP_THRESHOLD: usize = 5;

// Percentage thresholds for color coding
const HIGH_PERCENTAGE_THRESHOLD: f64 = 50.0;
const MEDIUM_PERCENTAGE_THRESHOLD: f64 = 10.0;

// Percentage thresholds for formatting precision
const LOW_PERCENTAGE_THRESHOLD: f64 = 0.1;
const MEDIUM_PRECISION_THRESHOLD: f64 = 1.0;

// Width for histogram labels (must be consistent for alignment)
const LABEL_WIDTH: usize = 12;

impl Reporter {
    /// Renders a histogram bar character based on percentage relative to the maximum percentage.
    ///
    /// Uses Unicode block characters to visually represent relative sizes:
    /// - Full blocks (█) for bars that fill the width
    /// - Partial blocks (▉▊▋▌▍▎▏) for very small values that would round to 0
    ///   This ensures even tiny percentages are visible and differentiated.
    ///
    /// This method scales bars based on percentages, ensuring the visual representation
    /// matches the displayed percentages proportionally.
    ///
    /// # Arguments
    ///
    /// * `percentage` - The percentage value for this bucket
    /// * `max_percentage` - The maximum percentage across all buckets (for scaling)
    /// * `bar_width` - The maximum width of the bar in characters
    ///
    /// # Returns
    ///
    /// A string containing the bar characters, or empty string if percentage is 0
    fn render_bar_from_percentage(
        percentage: f64,
        max_percentage: f64,
        bar_width: usize,
    ) -> String {
        if percentage <= 0.0 {
            return String::new();
        }

        // Calculate bar length (both integer and fractional parts) based on percentage
        let bar_length_fractional = if max_percentage > 0.0 {
            (percentage / max_percentage) * bar_width as f64
        } else {
            0.0
        };
        let bar_length = bar_length_fractional as usize;

        // Render bar based on calculated length
        if bar_length >= bar_width {
            // Full width bar
            "█".repeat(bar_width)
        } else if bar_length >= 1 {
            // One or more full blocks
            "█".repeat(bar_length)
        } else {
            // When bar_length rounds to 0, use fractional part to show relative size
            // This ensures different buckets show different visual indicators
            let fractional = bar_length_fractional.fract();
            match fractional {
                f if f >= 0.875 => "▉".to_string(), // Left seven-eighths block
                f if f >= 0.75 => "▊".to_string(),  // Left three-quarters block
                f if f >= 0.625 => "▋".to_string(), // Left five-eighths block
                f if f >= 0.5 => "▌".to_string(),   // Left half block
                f if f >= 0.375 => "▍".to_string(), // Left three-eighths block
                f if f >= 0.25 => "▎".to_string(),  // Left one-quarter block
                f if f >= 0.125 => "▏".to_string(), // Left one-eighth block
                _ => "▏".to_string(),               // Very tiny - still show something
            }
        }
    }

    /// Formats a percentage value with appropriate precision based on magnitude.
    ///
    /// Smaller percentages get more decimal places to show meaningful differences:
    /// - < 0.1%: 3 decimal places (e.g., "0.003%")
    /// - < 1.0%: 2 decimal places (e.g., "0.34%")
    /// - >= 1.0%: 1 decimal place (e.g., "69.2%")
    ///
    /// # Arguments
    ///
    /// * `percentage` - The percentage value to format
    ///
    /// # Returns
    ///
    /// A formatted string with 5 characters width (for alignment)
    fn format_percentage(percentage: f64) -> String {
        if percentage < LOW_PERCENTAGE_THRESHOLD {
            format!("{:5.3}%", percentage)
        } else if percentage < MEDIUM_PRECISION_THRESHOLD {
            format!("{:5.2}%", percentage)
        } else {
            format!("{:5.1}%", percentage)
        }
    }

    /// Formats a count value for display, using "k" suffix for thousands.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(Reporter::format_count(100), "    100");
    /// assert_eq!(Reporter::format_count(1000), "     1k");
    /// assert_eq!(Reporter::format_count(5000), "     5k");
    /// ```
    fn format_count(count: usize) -> String {
        if count >= 1000 {
            format!("{:>6}k", count / 1000)
        } else {
            format!("{:>7}", count)
        }
    }

    /// Returns a color-coded label based on the percentage value, padded to a fixed width.
    ///
    /// Color coding helps quickly identify performance characteristics:
    /// - Green: > 50% (most packets fall in this range)
    /// - Cyan: > 10% (significant portion)
    /// - Normal: <= 10% (minor portion)
    ///
    /// The label is padded to `LABEL_WIDTH` characters before applying colors to ensure
    /// consistent alignment when ANSI color codes are added.
    ///
    /// # Arguments
    ///
    /// * `label` - The label text to colorize
    /// * `percentage` - The percentage value used to determine color
    ///
    /// # Returns
    ///
    /// A colorized string padded to the specified width
    fn colorize_label(label: &str, percentage: f64) -> String {
        // Pad the label first to ensure consistent width, then apply colors
        let padded_label = format!("{:>width$}", label, width = LABEL_WIDTH);
        if percentage > HIGH_PERCENTAGE_THRESHOLD {
            padded_label.green().to_string()
        } else if percentage > MEDIUM_PERCENTAGE_THRESHOLD {
            padded_label.cyan().to_string()
        } else {
            padded_label.to_string()
        }
    }

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
        println!();

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

            if !found && latency_us >= OUTLIER_THRESHOLD_US {
                outliers += 1;
            }
        }

        // Calculate percentages first to find max percentage for bar scaling
        let mut percentages = Vec::new();
        for count in &bucket_counts {
            let percentage = (*count as f64 / total_packets as f64) * 100.0;
            percentages.push(percentage);
        }
        let max_percentage = percentages.iter().fold(0.0f64, |a, &b| a.max(b));

        // Print each bucket
        for (i, &(_, _, label)) in buckets.iter().enumerate() {
            let count = bucket_counts[i];
            if count == 0 && i > EMPTY_BUCKET_SKIP_THRESHOLD {
                continue; // Skip empty buckets beyond 100µs for cleaner output
            }

            let percentage = percentages[i];
            // Scale bars based on percentage, not count, to match displayed percentages
            let bar =
                Self::render_bar_from_percentage(percentage, max_percentage, HISTOGRAM_BAR_WIDTH);
            let label_colored = Self::colorize_label(label, percentage);
            let pct_str = Self::format_percentage(percentage);

            println!(
                "  {}:  {:30} {} ({:7} packets)",
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
            let outlier_bar =
                Self::render_bar_from_percentage(percentage, max_percentage, HISTOGRAM_BAR_WIDTH);
            let pct_str = Self::format_percentage(percentage);

            // Pad outlier label to match bucket label width
            let outlier_label = format!("{:>width$}", ">10 ms", width = LABEL_WIDTH);
            let outlier_label_colored = outlier_label.red().bold();

            println!(
                "  {}:  {:30} {} ({:7} packets) ← MAX: {:.1}ms",
                outlier_label_colored,
                outlier_bar,
                pct_str,
                Self::format_count(outliers),
                max_ms
            );
        }

        Ok(())
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
