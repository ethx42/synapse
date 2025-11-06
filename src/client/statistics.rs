use crate::client::constants::*;
use crate::client::error::{ClientError, Result};
use hdrhistogram::Histogram;
use tracing::{debug, warn};

/// Statistics calculator using HDR histogram
pub struct Statistics {
    hist: Histogram<u64>,
    real_min: u64,
    real_max: u64,
    clamped_count: usize,
}

impl Statistics {
    /// Create a new Statistics instance from latency measurements
    pub fn new(latencies: &[u64]) -> Result<Self> {
        debug!(
            sample_count = latencies.len(),
            "Creating statistics from latency measurements"
        );
        let mut hist = Histogram::<u64>::new_with_bounds(
            HISTOGRAM_LOW_BOUND_NS,
            HISTOGRAM_HIGH_BOUND_NS,
            HISTOGRAM_SIGNIFICANT_DIGITS,
        )
        .map_err(|e| ClientError::Measurement(format!("Failed to create histogram: {}", e)))?;

        let mut real_min = u64::MAX;
        let mut real_max = 0;
        let mut clamped_count = 0;

        for &latency in latencies {
            real_min = real_min.min(latency);
            real_max = real_max.max(latency);

            let clamped = latency.clamp(HISTOGRAM_LOW_BOUND_NS, HISTOGRAM_HIGH_BOUND_NS);

            if latency != clamped {
                clamped_count += 1;
            }

            hist.record(clamped).map_err(|e| {
                warn!(latency = latency, error = %e, "Failed to record latency");
                ClientError::Measurement(format!("Failed to record latency: {}", e))
            })?;
        }

        let result = Self {
            hist,
            real_min: if real_min == u64::MAX { 0 } else { real_min },
            real_max,
            clamped_count,
        };

        if clamped_count > 0 {
            warn!(
                clamped_count = clamped_count,
                total_count = latencies.len(),
                "Some latency values were clamped to histogram bounds"
            );
        }

        debug!(
            min_ns = result.real_min,
            max_ns = result.real_max,
            mean_ns = result.mean(),
            clamped_count = clamped_count,
            "Statistics calculated successfully"
        );

        Ok(result)
    }

    /// Get the mean latency
    pub fn mean(&self) -> f64 {
        self.hist.mean()
    }

    /// Get the minimum latency (unclamped)
    pub fn min(&self) -> u64 {
        self.real_min
    }

    /// Get the maximum latency (unclamped)
    pub fn max(&self) -> u64 {
        self.real_max
    }

    /// Get a percentile value
    pub fn percentile(&self, quantile: f64) -> u64 {
        self.hist.value_at_quantile(quantile)
    }

    /// Get the number of values that were clamped
    pub fn clamped_count(&self) -> usize {
        self.clamped_count
    }

    /// Get the total count of measurements
    pub fn count(&self) -> u64 {
        self.hist.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_calculation() -> Result<()> {
        let latencies = vec![1000, 2000, 3000, 4000, 5000];
        let stats = Statistics::new(&latencies)?;

        assert_eq!(stats.min(), 1000);
        assert_eq!(stats.max(), 5000);
        assert!(stats.mean() > 0.0);
        assert_eq!(stats.count(), 5);
        Ok(())
    }
}
