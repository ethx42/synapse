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

    let mut connected = false;
    let mut batch_received = 0u64;
    let mut batch_sent = 0u64;
    const BATCH_SIZE: u64 = 100;

    // Infinite loop: receive and immediately echo back
    loop {
        if !connected {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    batch_received += 1;

                    if let Err(e) = socket.connect(src) {
                        eprintln!("Failed to connect to {}: {}", src, e);
                        counters.increment_error();
                        continue;
                    }
                    connected = true;

                    match socket.send(&buf[..len]) {
                        Ok(_) => {
                            batch_sent += 1;
                        }
                        Err(e) => {
                            counters.increment_error();
                            eprintln!("Failed to send: {}", e);
                        }
                    }
                }
                Err(e) => {
                    counters.increment_error();
                    eprintln!("Failed to receive: {}", e);
                }
            }
        } else {
            match socket.recv(&mut buf) {
                Ok(len) => {
                    batch_received += 1;

                    // Immediately send back using connected socket
                    match socket.send(&buf[..len]) {
                        Ok(_) => {
                            batch_sent += 1;

                            if batch_sent >= BATCH_SIZE {
                                counters.add_received(batch_received);
                                counters.add_sent(batch_sent);
                                batch_received = 0;
                                batch_sent = 0;
                            }
                        }
                        Err(e) => {
                            if batch_received > 0 || batch_sent > 0 {
                                counters.add_received(batch_received);
                                counters.add_sent(batch_sent);
                                batch_received = 0;
                                batch_sent = 0;
                            }
                            counters.increment_error();
                            eprintln!("Failed to send: {}", e);
                            connected = false;
                        }
                    }
                }
                Err(e) => {
                    if batch_received > 0 || batch_sent > 0 {
                        counters.add_received(batch_received);
                        counters.add_sent(batch_sent);
                        batch_received = 0;
                        batch_sent = 0;
                    }
                    counters.increment_error();
                    eprintln!("Failed to receive: {}", e);
                    connected = false;
                }
            }
        }
    }
}
