use std::net::UdpSocket;
use std::process;
use synapse::server::ServerMonitor;

fn main() {
    let addr = "0.0.0.0:8080";

    // Bind the UDP socket
    let socket = match UdpSocket::bind(addr) {
        Ok(s) => {
            println!("Synapse server listening on {}", addr);
            s
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            process::exit(1);
        }
    };

    // Initialize server monitor (updates every 100ms for smooth display)
    let monitor = ServerMonitor::new(100);
    let counters = monitor.counters();

    // Start background display thread (non-blocking)
    monitor.start_display();

    // Pre-allocate a single receive buffer outside the loop (64 bytes is sufficient)
    let mut buf = [0u8; 64];

    println!("Ready to echo packets...");

    // Infinite loop: receive and immediately echo back
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                // Increment received counter (atomic, lock-free, minimal overhead)
                counters.increment_received();

                // Immediately send back the exact same payload
                match socket.send_to(&buf[..len], src) {
                    Ok(_) => {
                        // Increment sent counter (atomic, lock-free, minimal overhead)
                        counters.increment_sent();
                    }
                    Err(e) => {
                        counters.increment_error();
                        eprintln!("Failed to send to {}: {}", src, e);
                    }
                }
            }
            Err(e) => {
                counters.increment_error();
                eprintln!("Failed to receive: {}", e);
            }
        }
    }
}
