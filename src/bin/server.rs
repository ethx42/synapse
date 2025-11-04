use std::net::UdpSocket;
use std::process;

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

    // Pre-allocate a single receive buffer outside the loop (64 bytes is sufficient)
    let mut buf = [0u8; 64];

    println!("Ready to echo packets...");

    // Infinite loop: receive and immediately echo back
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                // Immediately send back the exact same payload
                if let Err(e) = socket.send_to(&buf[..len], src) {
                    eprintln!("Failed to send to {}: {}", src, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to receive: {}", e);
            }
        }
    }
}

