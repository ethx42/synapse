use crate::client::constants::*;
use crate::client::error::{ClientError, Result};
use crate::client::visualizer::OsiVisualizer;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};
use tracing::debug;

/// Progress tracker with live statistics and OSI visualization
pub struct ProgressTracker {
    pb: ProgressBar,
    visualizer: OsiVisualizer,
    last_update: Instant,
    update_interval: usize,
    last_stats_message: String,
    last_metrics_lines: Vec<String>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(packet_count: usize, update_interval: usize) -> Result<Self> {
        debug!(
            packet_count = packet_count,
            update_interval = update_interval,
            "Creating progress tracker"
        );
        let pb = ProgressBar::new(packet_count as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{msg}\n{bar:40.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]",
            )
            .map_err(|e| {
                ClientError::Measurement(format!("Failed to create progress style: {}", e))
            })?
            .progress_chars("█░"),
        );
        pb.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));

        Ok(Self {
            pb,
            visualizer: OsiVisualizer::new(),
            last_update: Instant::now(),
            update_interval,
            last_stats_message: String::new(),
            last_metrics_lines: Vec::new(),
        })
    }

    /// Update progress and live statistics
    pub fn update(
        &mut self,
        latencies: &[u64],
        start_time: Instant,
        packet_index: usize,
    ) -> Result<()> {
        self.pb.inc(1);

        // Advance OSI animation on sampled packets (lightweight operation)
        let should_advance = self.visualizer.should_update(packet_index);
        let mut should_update_display = false;

        if should_advance {
            self.visualizer.advance();
            // When animation advances, update display to show the new state
            // This ensures smooth animation without expensive stats calculations
            should_update_display = true;
        }

        // Update live stats less frequently to avoid performance overhead
        // Full stats update (with expensive calculations) happens at configured intervals
        let should_update_stats = (packet_index + 1).is_multiple_of(self.update_interval)
            || self.last_update.elapsed().as_millis() > LIVE_STATS_UPDATE_INTERVAL_MS as u128;

        // Update display when animation advances OR when full stats update is due
        if should_update_stats {
            if !latencies.is_empty() {
                // Full update with expensive stats calculations
                self.update_live_stats(latencies, start_time)?;
                self.last_update = Instant::now();
            }
        } else if should_update_display {
            // Lightweight update: update OSI visualization, reuse last stats
            // This allows smooth animation without expensive recalculations
            self.update_osi_display_only()?;
        }

        Ok(())
    }

    /// Update only the OSI visualization display (lightweight, reuse last stats)
    fn update_osi_display_only(&mut self) -> Result<()> {
        // Render OSI visualization
        let osi_viz = self.visualizer.render();
        let osi_lines: Vec<&str> = osi_viz.lines().collect();

        // Reuse last metrics lines if available, otherwise just show OSI
        if !self.last_metrics_lines.is_empty() {
            // Combine cached metrics with updated OSI visualization
            let mut combined = Vec::new();
            let max_lines = self.last_metrics_lines.len().max(osi_lines.len());
            for i in 0..max_lines {
                let metric_part = if i < self.last_metrics_lines.len() {
                    self.last_metrics_lines[i].clone()
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

            let msg = combined.join("\n");
            self.pb.set_message(msg);
        } else {
            // No stats yet, just show OSI visualization
            let mut combined = Vec::new();
            for line in osi_lines {
                combined.push(format!("{:<25}{}", "", line));
            }
            let msg = combined.join("\n");
            self.pb.set_message(msg);
        }
        Ok(())
    }

    /// Update the live statistics display
    fn update_live_stats(&mut self, latencies: &[u64], start_time: Instant) -> Result<()> {
        let last = latencies
            .last()
            .ok_or_else(|| ClientError::Measurement("No latencies available".into()))?;
        let mean = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;

        // Calculate a quick p99 estimate for live feedback
        // Only calculate if we have enough samples to make it meaningful
        let p99 = if latencies.len() > 10 {
            let mut sorted = latencies.to_vec();
            sorted.sort_unstable();
            let p99_idx = (sorted.len() as f64 * 0.99) as usize;
            *sorted.get(p99_idx).unwrap_or(&0)
        } else {
            // For small samples, use max as approximation
            *latencies.iter().max().unwrap_or(&0)
        };

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
        let p99_ms = p99 as f64 / 1_000_000.0;

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
        let osi_viz = self.visualizer.render();
        let osi_lines: Vec<&str> = osi_viz.lines().collect();

        // Build combined display with metrics on left, OSI on right
        // Calculate plain text lengths to ensure consistent width (25 chars visible)
        let last_plain = format!("→ {:.3}ms", last_ms);
        let mean_plain = format!("Mean: {:.3}ms", mean_ms);
        let p99_plain = format!("P99: {:.3}ms", p99_ms);
        let rate_plain = format!("Rate: {:.1}k pkt/s", rate / 1000.0);

        // Build metrics with proper padding BEFORE combining with colored values
        // This ensures each line is exactly 25 visible characters
        let metrics_lines = vec![
            format!(
                "→ {}ms{}",
                last_color,
                " ".repeat(25_usize.saturating_sub(last_plain.len()))
            ),
            format!(
                "Mean: {}ms{}",
                mean_color,
                " ".repeat(25_usize.saturating_sub(mean_plain.len()))
            ),
            format!("{:<25}", p99_plain),
            format!("{:<25}", rate_plain),
        ];

        // Cache the metrics lines for lightweight updates (avoids byte-slicing ANSI codes)
        self.last_metrics_lines = metrics_lines.clone();

        let mut combined = Vec::new();

        // Combine metrics and OSI lines side by side
        let max_lines = metrics_lines.len().max(osi_lines.len());
        for i in 0..max_lines {
            let metric_part = if i < metrics_lines.len() {
                // Use the pre-formatted metric line directly (already 25 chars wide)
                metrics_lines[i].clone()
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

        // Use indicatif's message field with newlines
        let msg = combined.join("\n");
        // Cache the message for reference
        self.last_stats_message = msg.clone();
        self.pb.set_message(msg);
        Ok(())
    }

    /// Finish the progress bar
    pub fn finish(&mut self) {
        self.pb.finish();
    }

    /// Final update of statistics before finishing
    pub fn final_update(&mut self, latencies: &[u64], start_time: Instant) -> Result<()> {
        if !latencies.is_empty() {
            self.update_live_stats(latencies, start_time)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_new() -> Result<()> {
        let tracker = ProgressTracker::new(100, 10)?;
        // Should create successfully - verify by checking it can be updated
        assert!(tracker.pb.length().unwrap() == 100);
        Ok(())
    }

    #[test]
    fn test_progress_tracker_update() -> Result<()> {
        let mut tracker = ProgressTracker::new(100, 10)?;
        let latencies = vec![1000, 2000, 3000];
        let start_time = Instant::now();

        // Update should succeed
        tracker.update(&latencies, start_time, 0)?;
        assert_eq!(tracker.pb.position(), 1);
        Ok(())
    }

    #[test]
    fn test_progress_tracker_final_update() -> Result<()> {
        let mut tracker = ProgressTracker::new(100, 10)?;
        let latencies = vec![1000, 2000, 3000];
        let start_time = Instant::now();

        tracker.final_update(&latencies, start_time)?;
        Ok(())
    }

    #[test]
    fn test_progress_tracker_finish() {
        let mut tracker = ProgressTracker::new(100, 10).unwrap();
        tracker.finish();
        // Should complete without error
    }
}
