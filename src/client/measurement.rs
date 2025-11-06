use crate::client::error::{ClientError, Result};
use crate::client::progress::ProgressTracker;
use crate::client::socket::NetworkSocket;
use crate::protocol::{Packet, SequenceNumber};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Represents a single measurement result
#[derive(Debug, Clone)]
pub struct Measurement {
    pub sequence: SequenceNumber,
    pub latency_ns: u64,
    pub timestamp: Instant,
}

/// Results from a complete measurement phase
#[derive(Debug, Clone)]
pub struct MeasurementResult {
    pub latencies: Vec<u64>,
    pub lost_packets: usize,
    pub total_packets: usize,
    pub elapsed: Duration,
}

/// Measure a single packet round-trip latency
pub fn measure_single_packet<S: NetworkSocket>(
    socket: &mut S,
    sequence: SequenceNumber,
) -> Result<Option<u64>> {
    let packet = Packet::new(sequence);
    let t1 = Instant::now();

    debug!("Sending packet");
    socket.send_packet(&packet)?;

    match socket.recv_packet() {
        Ok(recv_packet) => {
            let t2 = Instant::now();

            if recv_packet.sequence == sequence {
                let latency_ns = (t2 - t1).as_nanos() as u64;
                debug!(latency_ns = latency_ns, "Packet received successfully");
                Ok(Some(latency_ns))
            } else {
                warn!(
                    expected = sequence.0,
                    received = recv_packet.sequence.0,
                    "Sequence mismatch"
                );
                Ok(None) // Sequence mismatch
            }
        }
        Err(ClientError::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
            debug!("Packet receive timeout");
            Ok(None) // Timeout
        }
        Err(e) => {
            warn!(error = %e, "Error receiving packet");
            Err(e)
        }
    }
}

/// Perform warmup phase to stabilize system conditions
///
/// This phase populates ARP tables, warms CPU/OS caches, and establishes
/// baseline network paths before measurement begins.
pub fn warmup_phase<S: NetworkSocket>(socket: &mut S, warmup_count: usize) -> Result<()> {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut spinner_idx = 0;
    let mut successful_packets = 0usize;
    let mut lost_packets = 0usize;

    for seq in 0..warmup_count {
        let sequence = SequenceNumber(seq as u64);

        // Send and receive, but discard results
        match measure_single_packet(socket, sequence) {
            Ok(Some(_)) => {
                successful_packets += 1;
                debug!(packet_num = seq + 1, "Warmup packet completed");
            }
            Ok(None) => {
                lost_packets += 1;
                warn!(packet_num = seq + 1, "Warmup packet lost or timed out");
            }
            Err(e) => {
                // Error occurred - return with context about how many packets were processed
                let actual_packets = successful_packets + lost_packets;
                return Err(ClientError::Measurement(format!(
                    "Warmup phase interrupted after {} packets ({} successful, {} lost): {}",
                    actual_packets, successful_packets, lost_packets, e
                )));
            }
        }

        // Update spinner every 10 packets for smooth animation
        if seq % 10 == 0 {
            print!(
                "\rWarming up {} ({}/{})",
                spinner_chars[spinner_idx],
                seq + 1,
                warmup_count
            );
            io::stdout().flush().map_err(ClientError::Io)?;
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();
        }
    }

    println!("\rWarming up ✓ ({}/{})", warmup_count, warmup_count);
    println!();
    Ok(())
}

/// Perform measurement phase and collect latency statistics
pub fn measurement_phase<S: NetworkSocket>(
    socket: &mut S,
    packet_count: usize,
    update_interval: usize,
) -> Result<MeasurementResult> {
    // Pre-allocate vectors
    let mut latencies = Vec::with_capacity(packet_count);
    let mut lost_packets = 0usize;

    let start_time = Instant::now();

    // Create progress tracker
    let mut progress = ProgressTracker::new(packet_count, update_interval)?;

    for i in 0..packet_count {
        let sequence = SequenceNumber(i as u64);

        match measure_single_packet(socket, sequence) {
            Ok(Some(latency_ns)) => {
                latencies.push(latency_ns);
                debug!(
                    packet_num = i + 1,
                    latency_ns = latency_ns,
                    "Measurement packet completed"
                );
            }
            Ok(None) => {
                lost_packets += 1;
                warn!(packet_num = i + 1, "Measurement packet lost or timed out");
            }
            Err(e) => {
                // Error occurred - return with context about how many packets were processed
                let actual_packets = latencies.len() + lost_packets;
                return Err(ClientError::Measurement(format!(
                    "Measurement phase interrupted after {} packets ({} successful, {} lost): {}",
                    actual_packets,
                    latencies.len(),
                    lost_packets,
                    e
                )));
            }
        }

        // Update progress
        progress.update(&latencies, start_time, i)?;
    }

    debug!(
        packets_received = latencies.len(),
        packets_lost = lost_packets,
        "Measurement phase completed"
    );

    // Final update and finish
    progress.final_update(&latencies, start_time)?;
    progress.finish();
    println!(); // Add blank line for separation

    let elapsed = start_time.elapsed();
    Ok(MeasurementResult {
        latencies,
        lost_packets,
        total_packets: packet_count,
        elapsed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::socket::MockNetworkSocket;
    use std::io::ErrorKind;

    #[test]
    fn test_measure_single_packet_success() -> Result<()> {
        let mut mock_socket = MockNetworkSocket::new();
        let seq = SequenceNumber(123);

        mock_socket
            .expect_send_packet()
            .times(1)
            .returning(|_| Ok(8));

        mock_socket
            .expect_recv_packet()
            .times(1)
            .returning(move || Ok(Packet::new(seq)));

        let result = measure_single_packet(&mut mock_socket, seq)?;
        assert!(result.is_some());
        assert!(result.unwrap() > 0); // Some latency measured
        Ok(())
    }

    #[test]
    fn test_measure_single_packet_sequence_mismatch() -> Result<()> {
        let mut mock_socket = MockNetworkSocket::new();
        let seq = SequenceNumber(123);
        let wrong_seq = SequenceNumber(456);

        mock_socket
            .expect_send_packet()
            .times(1)
            .returning(|_| Ok(8));

        mock_socket
            .expect_recv_packet()
            .times(1)
            .returning(move || Ok(Packet::new(wrong_seq)));

        let result = measure_single_packet(&mut mock_socket, seq)?;
        assert!(result.is_none()); // Sequence mismatch
        Ok(())
    }

    #[test]
    fn test_measure_single_packet_timeout() -> Result<()> {
        let mut mock_socket = MockNetworkSocket::new();
        let seq = SequenceNumber(123);

        mock_socket
            .expect_send_packet()
            .times(1)
            .returning(|_| Ok(8));

        mock_socket
            .expect_recv_packet()
            .times(1)
            .returning(|| Err(ClientError::Io(std::io::Error::from(ErrorKind::TimedOut))));

        let result = measure_single_packet(&mut mock_socket, seq)?;
        assert!(result.is_none()); // Timeout
        Ok(())
    }

    #[test]
    fn test_measure_single_packet_send_error() {
        let mut mock_socket = MockNetworkSocket::new();
        let seq = SequenceNumber(123);

        mock_socket.expect_send_packet().times(1).returning(|_| {
            Err(ClientError::Io(std::io::Error::from(
                ErrorKind::ConnectionRefused,
            )))
        });

        let result = measure_single_packet(&mut mock_socket, seq);
        assert!(result.is_err());
    }
}
