use std::net::UdpSocket;
use std::process;
use std::time::{Duration, Instant};
use synapse::server::ServerMonitor;

fn main() {
    let addr = "0.0.0.0:8080";

    // Bind the UDP socket
    let mut socket = match UdpSocket::bind(addr) {
        Ok(s) => {
            println!("Synapse server listening on {}", addr);
            s
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            process::exit(1);
        }
    };

    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("Failed to set read timeout");

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
    let mut last_activity = Instant::now();
    const BATCH_SIZE: u64 = 100;
    const IDLE_DISCONNECT_MS: u64 = 200;

    loop {
        if !connected {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    batch_received += 1;
                    last_activity = Instant::now();

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
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    counters.increment_error();
                    eprintln!("Failed to receive: {}", e);
                }
            }
        } else {
            match socket.recv(&mut buf) {
                Ok(len) => {
                    batch_received += 1;
                    last_activity = Instant::now();

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

                            drop(socket);
                            socket = UdpSocket::bind(addr).expect("Failed to rebind socket");
                            socket
                                .set_read_timeout(Some(Duration::from_millis(100)))
                                .expect("Failed to set read timeout");
                            connected = false;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if last_activity.elapsed().as_millis() >= IDLE_DISCONNECT_MS as u128 {
                        if batch_received > 0 || batch_sent > 0 {
                            counters.add_received(batch_received);
                            counters.add_sent(batch_sent);
                            batch_received = 0;
                            batch_sent = 0;
                        }

                        drop(socket);
                        socket = UdpSocket::bind(addr).expect("Failed to rebind socket");
                        socket
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .expect("Failed to set read timeout");
                        connected = false;
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

                    drop(socket);
                    socket = UdpSocket::bind(addr).expect("Failed to rebind socket");
                    socket
                        .set_read_timeout(Some(Duration::from_millis(100)))
                        .expect("Failed to set read timeout");
                    connected = false;
                }
            }
        }
    }
}
