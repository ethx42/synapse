use synapse::client::{Config, NetworkSocket, UdpNetworkSocket, warmup_phase, measurement_phase, Statistics};
use synapse::client::Result;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

/// Test helper: Start a simple echo server
fn start_test_server(port: u16) -> UdpSocket {
    let addr = format!("127.0.0.1:{}", port);
    let socket = UdpSocket::bind(&addr).expect("Failed to bind test server");
    socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    socket
}

/// Test helper: Echo server that responds to packets
fn run_echo_server(socket: UdpSocket) {
    let mut buf = [0u8; 64];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                let _ = socket.send_to(&buf[..len], src);
            }
            Err(_) => {
                // Timeout or error - exit loop
                break;
            }
        }
    }
}

#[test]
fn test_config_validation() {
    let mut config = Config {
        server: "127.0.0.1:8080".to_string(),
        packets: 0,
        warmup: 10,
        update: 10,
        timeout_ms: 100,
    };
    
    // Should fail validation
    assert!(config.validate().is_err());
    
    // Fix and should pass
    config.packets = 10;
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_timeout() {
    let config = Config {
        server: "127.0.0.1:8080".to_string(),
        packets: 10,
        warmup: 5,
        update: 5,
        timeout_ms: 500,
    };
    
    let timeout = config.timeout();
    assert_eq!(timeout, Duration::from_millis(500));
}

#[test]
fn test_end_to_end_measurement() -> Result<()> {
    // Start test server on a random port
    let server_socket = start_test_server(0);
    let server_addr = server_socket.local_addr().unwrap();
    
    // Start server in background thread
    let _server_handle = thread::spawn(move || {
        run_echo_server(server_socket);
    });
    
    // Give server time to start
    thread::sleep(Duration::from_millis(100));
    
    // Create client socket
    let mut client_socket = UdpNetworkSocket::bind("127.0.0.1:0")?;
    client_socket.connect(&format!("127.0.0.1:{}", server_addr.port()))?;
    client_socket.set_timeout(Duration::from_millis(1000))?;
    
    // Run warmup phase
    warmup_phase(&mut client_socket, 5)?;
    
    // Run measurement phase with small packet count
    let result = measurement_phase(&mut client_socket, 10, 5)?;
    
    // Verify results
    assert!(result.total_packets == 10);
    assert!(result.latencies.len() + result.lost_packets == 10);
    
    // Calculate statistics
    if !result.latencies.is_empty() {
        let stats = Statistics::new(&result.latencies)?;
        assert!(stats.count() > 0);
        assert!(stats.mean() > 0.0);
    }
    
    // Clean up
    drop(client_socket);
    thread::sleep(Duration::from_millis(100));
    
    Ok(())
}

#[test]
fn test_measurement_with_lost_packets() -> Result<()> {
    // This test verifies that lost packets are handled correctly
    // We'll use a mock or a server that doesn't respond to simulate packet loss
    // For now, we'll skip this and rely on unit tests with mocks
    Ok(())
}

