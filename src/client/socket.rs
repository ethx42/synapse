use crate::client::error::{ClientError, Result};
use crate::protocol::{Packet, PACKET_SIZE};
use std::net::UdpSocket;
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

/// UDP-based implementation of NetworkSocket
#[derive(Debug)]
pub struct UdpNetworkSocket {
    socket: UdpSocket,
}

impl UdpNetworkSocket {
    /// Bind to a local address
    pub fn bind(addr: &str) -> Result<Self> {
        debug!(addr = addr, "Binding UDP socket");
        let socket = UdpSocket::bind(addr).map_err(|e| {
            warn!(error = %e, "Failed to bind socket");
            ClientError::Socket(format!("Failed to bind to {}: {}", addr, e))
        })?;
        debug!("Socket bound successfully");
        Ok(Self { socket })
    }

    /// Connect to a remote address
    pub fn connect(&self, addr: &str) -> Result<()> {
        debug!(addr = addr, "Connecting UDP socket");
        self.socket.connect(addr).map_err(|e| {
            warn!(error = %e, "Failed to connect socket");
            ClientError::Socket(format!("Failed to connect to {}: {}", addr, e))
        })?;
        debug!("Socket connected successfully");
        Ok(())
    }
}

impl NetworkSocket for UdpNetworkSocket {
    fn send_packet(&self, packet: &Packet) -> Result<usize> {
        let buf = packet.encode();
        let bytes_sent = self.socket.send(&buf).map_err(|e| {
            warn!(error = %e, "Failed to send packet");
            ClientError::Io(e)
        })?;
        debug!(
            bytes_sent = bytes_sent,
            sequence = packet.sequence.0,
            "Packet sent"
        );
        Ok(bytes_sent)
    }

    fn recv_packet(&mut self) -> Result<Packet> {
        let mut buf = [0u8; PACKET_SIZE];
        let len = self.socket.recv(&mut buf).map_err(|e| {
            debug!(error = %e, "Failed to receive packet");
            ClientError::Io(e)
        })?;
        let packet = Packet::decode(&buf[..len])?;
        debug!(
            sequence = packet.sequence.0,
            bytes_received = len,
            "Packet received"
        );
        Ok(packet)
    }

    fn set_timeout(&self, timeout: Duration) -> Result<()> {
        debug!(timeout_ms = timeout.as_millis(), "Setting socket timeout");
        self.socket.set_read_timeout(Some(timeout)).map_err(|e| {
            warn!(error = %e, "Failed to set timeout");
            ClientError::Socket(format!("Failed to set timeout: {}", e))
        })?;
        debug!("Timeout set successfully");
        Ok(())
    }
}

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
    fn test_udp_socket_bind() {
        let socket = UdpNetworkSocket::bind("127.0.0.1:0");
        assert!(socket.is_ok());
    }

    #[test]
    fn test_send_recv_packet() -> Result<()> {
        // This would require a test server, so we'll skip it for now
        // It will be tested in integration tests
        Ok(())
    }
}

#[cfg(test)]
pub use tests::MockNetworkSocket;
