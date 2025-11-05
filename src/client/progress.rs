use indicatif::{ProgressBar, ProgressStyle};
use crate::client::visualizer::OsiVisualizer;
use crate::client::error::{ClientError, Result};
use crate::client::constants::*;
use std::time::{Duration, Instant};
use colored::*;

/// Progress tracker with live statistics and OSI visualization
pub struct ProgressTracker {
    pb: ProgressBar,
    visualizer: OsiVisualizer,
    last_update: Instant,
    update_interval: usize,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(packet_count: usize, update_interval: usize) -> Result<Self> {
        let pb = ProgressBar::new(packet_count as u64);
        pb.set_style(
            ProgressStyle::with_template("{msg}\n{bar:40.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]")
                .map_err(|e| ClientError::Measurement(format!("Failed to create progress style: {}", e)))?
                .progress_chars("█░")
        );
        pb.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));

        Ok(Self {
            pb,
            visualizer: OsiVisualizer::new(),
            last_update: Instant::now(),
            update_interval,
        })
    }

    /// Update progress and live statistics
    pub fn update(&mut self, latencies: &[u64], start_time: Instant, packet_index: usize) -> Result<()> {
        self.pb.inc(1);
        
        // Update live stats less frequently to avoid clutter
        if (packet_index + 1) % self.update_interval == 0 
            || self.last_update.elapsed().as_millis() > LIVE_STATS_UPDATE_INTERVAL_MS as u128 
        {
            if !latencies.is_empty() {
                self.update_live_stats(latencies, start_time)?;
            }
            self.last_update = Instant::now();
        }
        
        // Advance OSI animation on sampled packets
        if self.visualizer.should_update(packet_index) {
            self.visualizer.advance();
        }
        
        Ok(())
    }

    /// Update the live statistics display
    fn update_live_stats(&self, latencies: &[u64], start_time: Instant) -> Result<()> {
        let last = latencies.last()
            .ok_or_else(|| ClientError::Measurement("No latencies available".into()))?;
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
        let osi_viz = self.visualizer.render();
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
        
        // Use indicatif's message field with newlines
        let msg = combined.join("\n");
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

