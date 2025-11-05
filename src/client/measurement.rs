use crate::client::socket::NetworkSocket;
use crate::client::error::{ClientError, Result};
use crate::client::progress::ProgressTracker;
use crate::protocol::{Packet, SequenceNumber};
use std::io::{self, Write};
use std::time::{Duration, Instant};

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

    socket.send_packet(&packet)?;

    match socket.recv_packet() {
        Ok(recv_packet) => {
            let t2 = Instant::now();
            
            if recv_packet.sequence == sequence {
                let latency_ns = (t2 - t1).as_nanos() as u64;
                Ok(Some(latency_ns))
            } else {
                Ok(None) // Sequence mismatch
            }
        }
        Err(ClientError::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
            Ok(None) // Timeout
        }
        Err(e) => Err(e),
    }
}

/// Perform warmup phase to stabilize network conditions
pub fn warmup_phase<S: NetworkSocket>(
    socket: &mut S,
    warmup_count: usize,
) -> Result<()> {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut spinner_idx = 0;

    for seq in 0..warmup_count {
        let sequence = SequenceNumber(seq as u64);
        
        // Send and receive, but discard results
        if measure_single_packet(socket, sequence)?.is_some() {
            // Packet received successfully
        }
        
        // Update spinner every 10 packets for smooth animation
        if seq % 10 == 0 {
            print!("\rWarming up {} ({}/{})", 
                spinner_chars[spinner_idx], 
                seq + 1, 
                warmup_count
            );
            io::stdout().flush()
                .map_err(|e| ClientError::Io(e))?;
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
        
        match measure_single_packet(socket, sequence)? {
            Some(latency_ns) => {
                latencies.push(latency_ns);
            }
            None => {
                lost_packets += 1;
            }
        }

        // Update progress
        progress.update(&latencies, start_time, i)?;
    }

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
            .returning(|| {
                Err(ClientError::Io(std::io::Error::from(ErrorKind::TimedOut)))
            });

        let result = measure_single_packet(&mut mock_socket, seq)?;
        assert!(result.is_none()); // Timeout
        Ok(())
    }

    #[test]
    fn test_measure_single_packet_send_error() {
        let mut mock_socket = MockNetworkSocket::new();
        let seq = SequenceNumber(123);

        mock_socket
            .expect_send_packet()
            .times(1)
            .returning(|_| Err(ClientError::Io(std::io::Error::from(ErrorKind::ConnectionRefused))));

        let result = measure_single_packet(&mut mock_socket, seq);
        assert!(result.is_err());
    }
}

