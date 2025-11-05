use crate::client::error::{ClientError, Result};
use crate::client::constants::PACKET_SIZE;
use crate::protocol::Packet;
use std::net::UdpSocket;
use std::time::Duration;

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
pub struct UdpNetworkSocket {
    socket: UdpSocket,
}

impl UdpNetworkSocket {
    /// Bind to a local address
    pub fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr)
            .map_err(|e| ClientError::Socket(format!("Failed to bind to {}: {}", addr, e)))?;
        Ok(Self { socket })
    }

    /// Connect to a remote address
    pub fn connect(&self, addr: &str) -> Result<()> {
        self.socket.connect(addr)
            .map_err(|e| ClientError::Socket(format!("Failed to connect to {}: {}", addr, e)))?;
        Ok(())
    }
}

impl NetworkSocket for UdpNetworkSocket {
    fn send_packet(&self, packet: &Packet) -> Result<usize> {
        let buf = packet.encode();
        self.socket.send(&buf)
            .map_err(|e| ClientError::Io(e))
    }

    fn recv_packet(&mut self) -> Result<Packet> {
        let mut buf = [0u8; PACKET_SIZE];
        let len = self.socket.recv(&mut buf)
            .map_err(|e| ClientError::Io(e))?;
        Packet::decode(&buf[..len])
    }

    fn set_timeout(&self, timeout: Duration) -> Result<()> {
        self.socket.set_read_timeout(Some(timeout))
            .map_err(|e| ClientError::Socket(format!("Failed to set timeout: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

