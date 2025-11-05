//! Constants used throughout the client application

/// Size of a packet in bytes
pub const PACKET_SIZE: usize = 8;

/// Sample rate for OSI layer animation (animate every Nth packet)
pub const OSI_ANIMATION_SAMPLE_RATE: usize = 1;

/// Progress bar tick interval in milliseconds
pub const PROGRESS_TICK_INTERVAL_MS: u64 = 100;

/// Live statistics update interval in milliseconds
pub const LIVE_STATS_UPDATE_INTERVAL_MS: u64 = 500;

/// Histogram lower bound in nanoseconds
pub const HISTOGRAM_LOW_BOUND_NS: u64 = 100;

/// Histogram upper bound in nanoseconds
pub const HISTOGRAM_HIGH_BOUND_NS: u64 = 100_000_000;

/// Histogram significant digits for precision
pub const HISTOGRAM_SIGNIFICANT_DIGITS: u8 = 3;

/// Pass threshold for mean latency in milliseconds
pub const PASS_THRESHOLD_MS: f64 = 1.0;

/// Excellent latency threshold in milliseconds
pub const EXCELLENT_LATENCY_MS: f64 = 0.5;

/// Acceptable latency threshold in milliseconds
pub const ACCEPTABLE_LATENCY_MS: f64 = 1.0;
