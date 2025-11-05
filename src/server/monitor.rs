//! Server monitoring and statistics display

use colored::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Idle detection threshold - time without packets before marking as idle (milliseconds).
///
/// This threshold balances responsiveness with avoiding false positives.
/// Reduced for faster detection when tests finish.
const IDLE_THRESHOLD_MS: u64 = 150;

/// Blink interval for the activity indicator (milliseconds).
///
/// Controls how fast the indicator blinks when packets are actively being received.
/// A value of 200ms provides visible feedback without being distracting.
const BLINK_INTERVAL_MS: u64 = 200;

/// Monitor for tracking server packet statistics with minimal performance impact.
///
/// Uses atomic counters for lock-free updates and updates the display
/// periodically in a background thread to avoid blocking the main receive loop.
pub struct ServerMonitor {
    packets_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    start_time: Instant,
    update_interval: Duration,
}

impl ServerMonitor {
    /// Create a new server monitor with the specified update interval.
    ///
    /// # Arguments
    ///
    /// * `update_interval_ms` - Display update interval in milliseconds
    pub fn new(update_interval_ms: u64) -> Self {
        Self {
            packets_received: Arc::new(AtomicU64::new(0)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            update_interval: Duration::from_millis(update_interval_ms),
        }
    }

    /// Get counters for use in the main receive loop.
    ///
    /// Returns a clone of the atomic counters that can be updated
    /// from the main thread without blocking.
    pub fn counters(&self) -> ServerCounters {
        ServerCounters {
            packets_received: Arc::clone(&self.packets_received),
            packets_sent: Arc::clone(&self.packets_sent),
            errors: Arc::clone(&self.errors),
        }
    }

    /// Start the background display thread.
    ///
    /// This spawns a separate thread that periodically updates the display
    /// without blocking the main receive loop.
    ///
    /// Performance note: The `thread::sleep()` call is in this BACKGROUND thread,
    /// not the main receive loop. The main receive loop has zero overhead from
    /// the display thread - it only does atomic counter increments which are
    /// lock-free and take nanoseconds.
    pub fn start_display(&self) {
        let packets_received = Arc::clone(&self.packets_received);
        let packets_sent = Arc::clone(&self.packets_sent);
        let errors = Arc::clone(&self.errors);
        let update_interval = self.update_interval;

        thread::spawn(move || {
            let mut last_received = 0u64;
            let mut last_packet_time = Instant::now();
            let mut blink_state = false;
            let mut last_blink_time = Instant::now();

            loop {
                thread::sleep(update_interval);

                let received = packets_received.load(Ordering::Relaxed);
                let sent = packets_sent.load(Ordering::Relaxed);
                let error_count = errors.load(Ordering::Relaxed);
                let now = Instant::now();

                // Detect if actively receiving packets
                let recent_received = received.saturating_sub(last_received);

                // Determine if server is idle (no packets in last IDLE_THRESHOLD_MS)
                // This threshold balances responsiveness with avoiding false positives
                let time_since_last_packet = now.duration_since(last_packet_time);
                let idle_threshold_duration = Duration::from_millis(IDLE_THRESHOLD_MS);
                let is_idle =
                    recent_received == 0 && time_since_last_packet >= idle_threshold_duration;

                // Update last packet time if we received new packets
                if recent_received > 0 {
                    last_packet_time = now;
                }

                // Blinking indicator synchronized with packet reception
                // Continuously blinks when active, static when idle
                if !is_idle {
                    // Continuous blinking when receiving packets
                    let blink_interval = Duration::from_millis(BLINK_INTERVAL_MS);
                    if now.duration_since(last_blink_time) >= blink_interval {
                        blink_state = !blink_state;
                        last_blink_time = now;
                    }
                } else {
                    // Static when idle
                    blink_state = false;
                }

                // Render indicator based on activity and blink state
                let indicator = Self::render_indicator(is_idle, blink_state);

                // Format and display status line
                Self::display_status_line(&indicator, is_idle, received, sent, error_count);

                last_received = received;
            }
        });
    }

    /// Renders the activity indicator based on current state.
    ///
    /// # Arguments
    ///
    /// * `is_idle` - Whether the server is currently idle
    /// * `blink_state` - Current blink state (true = ON, false = OFF)
    ///
    /// # Returns
    ///
    /// A colored string representing the indicator
    fn render_indicator(is_idle: bool, blink_state: bool) -> String {
        if is_idle {
            // Static gray block when idle
            "░".normal().to_string()
        } else if blink_state {
            // Blinking ON state - red filled block
            "█".red().bold().to_string()
        } else {
            // Blinking OFF state - red outlined block (light shade)
            "░".red().to_string()
        }
    }

    /// Displays the status line with current server statistics.
    ///
    /// # Arguments
    ///
    /// * `indicator` - The activity indicator string
    /// * `is_idle` - Whether the server is currently idle
    /// * `received` - Total packets received
    /// * `sent` - Total packets sent
    /// * `error_count` - Total errors encountered
    fn display_status_line(
        indicator: &str,
        is_idle: bool,
        received: u64,
        sent: u64,
        error_count: u64,
    ) {
        let status = if is_idle { "IDLE" } else { "ACTIVE" };
        print!(
            "\r{} [{}] Received: {} | Sent: {} | Errors: {}",
            indicator, status, received, sent, error_count
        );
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }

    /// Get final statistics.
    pub fn stats(&self) -> ServerStats {
        let elapsed = self.start_time.elapsed();
        let received = self.packets_received.load(Ordering::Relaxed);
        let sent = self.packets_sent.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        ServerStats {
            packets_received: received,
            packets_sent: sent,
            errors,
            elapsed,
        }
    }
}

/// Lightweight counters for updating statistics from the main receive loop.
///
/// These use atomic operations which are lock-free and have minimal overhead.
pub struct ServerCounters {
    packets_received: Arc<AtomicU64>,
    packets_sent: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
}

impl ServerCounters {
    /// Increment the received packets counter.
    ///
    /// Uses `Relaxed` ordering which is sufficient for simple counters
    /// and provides the best performance.
    #[inline]
    pub fn increment_received(&self) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the sent packets counter.
    #[inline]
    pub fn increment_sent(&self) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the errors counter.
    #[inline]
    pub fn increment_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// Final server statistics.
pub struct ServerStats {
    pub packets_received: u64,
    pub packets_sent: u64,
    pub errors: u64,
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_monitor_creation() {
        let monitor = ServerMonitor::new(100);
        assert_eq!(monitor.update_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_counters() {
        let monitor = ServerMonitor::new(100);
        let counters = monitor.counters();

        counters.increment_received();
        counters.increment_sent();
        counters.increment_error();

        let stats = monitor.stats();
        assert_eq!(stats.packets_received, 1);
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.errors, 1);
    }

    #[test]
    fn test_counter_performance() {
        let monitor = ServerMonitor::new(100);
        let counters = monitor.counters();

        let start = Instant::now();
        for _ in 0..1_000_000 {
            counters.increment_received();
        }
        let elapsed = start.elapsed();

        // Should complete in under 100ms (very fast atomic operations)
        assert!(elapsed.as_millis() < 100);
        assert_eq!(monitor.stats().packets_received, 1_000_000);
    }
}
