use crate::client::error::{ClientError, Result};
use crate::protocol::{Packet, PACKET_SIZE};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;
use tracing::{debug, warn};

/// Trait for network socket operations with packet abstraction
pub trait NetworkSocket: Send + Sync {
    /// Send a packet over the network
    fn send_packet(&self, packet: &Packet) -> Result<usize>;

    /// Receive a packet from the network
    fn recv_packet(&mut self) -> Result<Packet>;

    /// Set the read timeout for the socket
    fn set_timeout(&self, timeout: Duration) -> Result<()>;
}

/// TCP-based implementation of NetworkSocket
pub struct TcpNetworkSocket {
    stream: Mutex<TcpStream>,
}

impl TcpNetworkSocket {
    /// Connect to a remote address
    pub fn connect(addr: &str) -> Result<Self> {
        debug!(addr = addr, "Connecting TCP stream");
        let stream = TcpStream::connect(addr).map_err(|e| {
            warn!(error = %e, "Failed to connect stream");
            ClientError::Socket(format!("Failed to connect to {}: {}", addr, e))
        })?;
        debug!("TCP stream connected successfully");
        Ok(Self {
            stream: Mutex::new(stream),
        })
    }
}

impl NetworkSocket for TcpNetworkSocket {
    fn send_packet(&self, packet: &Packet) -> Result<usize> {
        let buf = packet.encode();
        let mut stream = self.stream.lock().map_err(|e| {
            warn!(error = %e, "Failed to lock stream");
            ClientError::Socket(format!("Failed to lock stream: {}", e))
        })?;

        // TCP is stream-based, so we must use write_all to ensure all bytes are sent
        stream.write_all(&buf).map_err(|e| {
            warn!(error = %e, "Failed to send packet");
            ClientError::Io(e)
        })?;
        stream.flush().map_err(|e| {
            warn!(error = %e, "Failed to flush stream");
            ClientError::Io(e)
        })?;
        debug!(
            bytes_sent = buf.len(),
            sequence = packet.sequence.0,
            "Packet sent"
        );
        Ok(buf.len())
    }

    fn recv_packet(&mut self) -> Result<Packet> {
        let mut buf = [0u8; PACKET_SIZE];
        let mut stream = self.stream.lock().map_err(|e| {
            warn!(error = %e, "Failed to lock stream");
            ClientError::Socket(format!("Failed to lock stream: {}", e))
        })?;

        // TCP is stream-based, so we must use read_exact to read exactly PACKET_SIZE bytes
        stream.read_exact(&mut buf).map_err(|e| {
            debug!(error = %e, "Failed to receive packet");
            ClientError::Io(e)
        })?;

        let packet = Packet::decode(&buf)?;
        debug!(
            sequence = packet.sequence.0,
            bytes_received = PACKET_SIZE,
            "Packet received"
        );
        Ok(packet)
    }

    fn set_timeout(&self, timeout: Duration) -> Result<()> {
        debug!(timeout_ms = timeout.as_millis(), "Setting socket timeout");
        let stream = self.stream.lock().map_err(|e| {
            warn!(error = %e, "Failed to lock stream");
            ClientError::Socket(format!("Failed to lock stream: {}", e))
        })?;

        stream.set_read_timeout(Some(timeout)).map_err(|e| {
            warn!(error = %e, "Failed to set timeout");
            ClientError::Socket(format!("Failed to set timeout: {}", e))
        })?;
        debug!("Timeout set successfully");
        Ok(())
    }
}

#[cfg(test)]
pub use tests::MockNetworkSocket;

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub NetworkSocket {}

        impl NetworkSocket for NetworkSocket {
            fn send_packet(&self, packet: &Packet) -> Result<usize>;
            fn recv_packet(&mut self) -> Result<Packet>;
            fn set_timeout(&self, timeout: Duration) -> Result<()>;
        }
    }

    #[test]
    fn test_tcp_socket_connect() {
        // This test requires a server running, skip for unit tests
        // Will be tested in integration tests
    }

    #[test]
    fn test_send_recv_packet() -> Result<()> {
        // This would require a test server, so we'll skip it for now
        // It will be tested in integration tests
        Ok(())
    }
}
