//! Client module for Synapse latency measurement tool

pub mod config;
pub mod constants;
pub mod error;
pub mod logging;
pub mod measurement;
pub mod progress;
pub mod reporter;
pub mod socket;
pub mod statistics;
pub mod visualizer;

pub use config::Config;
pub use constants::*;
pub use error::{ClientError, Result};
pub use logging::init_logging;
pub use measurement::{
    measure_single_packet, measurement_phase, warmup_phase, Measurement, MeasurementResult,
};
pub use progress::ProgressTracker;
pub use reporter::Reporter;
pub use socket::{NetworkSocket, UdpNetworkSocket};
pub use statistics::Statistics;
pub use visualizer::OsiVisualizer;
