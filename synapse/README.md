# Synapse

A bare-metal diagnostic tool that measures a system's true stimulus-response reflex time at the network level.

## Overview

Synapse consists of two components:

- **Server**: A minimal UDP echo server with zero-allocation hot path
- **Client**: A precision timing client that measures round-trip network latency with sub-millisecond accuracy

## Getting Started (For Rust Beginners)

If you're new to Rust and Cargo, follow these step-by-step instructions to get Synapse running on your system.

### Step 1: Install Rust and Cargo

Rust comes with Cargo (Rust's package manager and build tool) included.

#### On macOS and Linux:

Open a terminal and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions. When prompted, press Enter to proceed with the default installation.

After installation completes, restart your terminal or run:

```bash
source $HOME/.cargo/env
```

#### On Windows:

1. Download the Rust installer from [https://rustup.rs/](https://rustup.rs/)
2. Run the downloaded `rustup-init.exe` file
3. Follow the on-screen instructions (accept defaults when prompted)
4. Restart your terminal/command prompt

### Step 2: Verify Installation

Check that Rust and Cargo are installed correctly:

```bash
rustc --version
cargo --version
```

You should see version numbers printed for both commands (e.g., `cargo 1.XX.X`).

### Step 3: Navigate to the Project

Open a terminal and navigate to the synapse project directory:

```bash
cd /path/to/synapse
```

Replace `/path/to/synapse` with the actual path where you downloaded or cloned this project.

### Step 4: Build the Project

Build both the server and client with optimizations:

```bash
cargo build --release
```

**What's happening:** Cargo will download all required dependencies and compile the project. This may take a few minutes the first time. The `--release` flag builds an optimized version for better performance.

You'll see output like:

```
   Compiling synapse v0.1.0
    Finished release [optimized] target(s) in 45.23s
```

### Step 5: Run the Server

Open a terminal window and start the server:

```bash
cargo run --release --bin server
```

You should see:

```
Synapse server listening on 0.0.0.0:8080
Ready to echo packets...
```

**Important:** Keep this terminal window open! The server needs to keep running.

### Step 6: Run the Client

Open a **second terminal window** (keep the server running in the first) and run the client:

```bash
cargo run --release --bin client -- --server 127.0.0.1:8080 --packets 1000000
```

**Note:** The `--` separator is required to pass arguments to the client (not to Cargo).

The client will connect to the server, run the test, and display results. You should see output showing progress, statistics, and a PASS/FAIL verdict.

### Quick Test Example

To run a quick test with fewer packets:

```bash
cargo run --release --bin client -- --packets 1000
```

### Common Issues and Solutions

#### "command not found: cargo"

- Make sure you completed Step 1 and restarted your terminal
- On macOS/Linux, try running: `source $HOME/.cargo/env`
- Check that `~/.cargo/bin` is in your PATH

#### "Address already in use" error

- Another program is using port 8080
- Stop the conflicting program or use a different port (future feature)

#### Very high latencies (> 1ms on localhost)

- Make sure both server and client are running on the same machine
- Close unnecessary background applications
- Consider applying OS-level tuning (see below)

#### Permission denied errors

- On Linux/macOS, some OS tuning commands require `sudo`
- The basic functionality doesn't require sudo - only advanced tuning does

## Building

```bash
cargo build --release
```

The release build uses aggressive optimizations:

- Link-Time Optimization (LTO)
- Single codegen unit
- Panic abort (no unwinding)

## Usage

### Start the Server

```bash
cargo run --release --bin server
```

The server listens on `0.0.0.0:8080` by default.

### Run the Client

```bash
cargo run --release --bin client -- --server 127.0.0.1:8080 --packets 1000000
```

Note: The `--` separator is required to pass arguments to the client program (not to Cargo itself).

#### Client Options

- `--server <IP:PORT>`: Server address (default: `127.0.0.1:8080`)
- `--packets <N>`: Number of packets to send (default: `10000`)
- `--warmup <N>`: Number of warmup packets (default: `200`)
- `--update <N>`: Dashboard update interval (default: `100`)
- `--timeout <ms>`: Socket timeout in milliseconds (default: `100`)

## Performance Target

**Goal: < 1ms average round-trip time**

The tool will report PASS/FAIL based on this threshold.

## OS-Level Tuning (Recommended)

For the most accurate measurements, apply these OS-level optimizations:

### CPU Governor

Set CPU governor to `performance` mode to prevent frequency scaling:

```bash
# Linux
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# macOS
# CPU governor is managed automatically; ensure no heavy background processes are running
```

### CPU Pinning

Pin server and client processes to specific CPU cores to reduce context switching:

```bash
# Linux - use taskset
taskset -c 0 ./target/release/server &
taskset -c 1 ./target/release/client

# macOS - use cpuset (requires root)
# Not directly supported; minimize background processes instead
```

### Network Interface Tuning (Linux)

```bash
# Increase UDP buffer sizes
sudo sysctl -w net.core.rmem_max=26214400
sudo sysctl -w net.core.wmem_max=26214400
sudo sysctl -w net.core.rmem_default=26214400
sudo sysctl -w net.core.wmem_default=26214400

# Disable interrupt coalescing for lower latency
sudo ethtool -C eth0 rx-usecs 0 tx-usecs 0
```

### Disable Power Management

```bash
# Linux
sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target

# macOS
sudo pmset -a disablesleep 1
```

### Run with Real-Time Priority (Advanced)

```bash
# Linux - requires CAP_SYS_NICE capability
sudo chrt -f 99 ./target/release/server &
sudo chrt -f 99 ./target/release/client
```

## Interpreting Results

The client outputs:

- **Statistical summary**: Mean, Min, Max, P50, P90, P99, P99.9
- **Histogram visualization**: ASCII distribution of latencies
- **Packet loss count**: Number of timeouts or sequence errors
- **Verdict**: PASS if mean < 1ms, otherwise FAIL

## Example Output

```
Warming up with 200 packets...
Running test with 10000 packets...
[Progress: 10000/10000] Current: 0.234ms | Mean: 0.187ms | P99: 0.421ms

=== Synapse Network Diagnostic Results ===
Packets sent:     10000
Packets lost:     0 (0.00%)

--- Latency Statistics (RTT) ---
Mean:             187.3 µs
Minimum:          142.1 µs
Maximum:          523.8 µs
P50 (median):     178.2 µs
P90:              245.7 µs
P99:              421.3 µs
P99.9:            498.2 µs

--- Latency Distribution ---
[Histogram visualization here]

✓ PASS: Mean latency (0.187ms) is below 1ms threshold
```

## Technical Details

### Why Blocking I/O?

Synapse uses `std::net` (blocking I/O) instead of async runtimes like Tokio to:

- Minimize runtime overhead
- Reduce jitter from executor scheduling
- Achieve deterministic timing

### Measurement Methodology

1. **Warm-up phase**: Sends warmup packets to populate ARP tables and warm CPU/OS caches
2. **Measurement phase**: Uses `Instant::now()` before send and immediately after receive
3. **Post-processing**: Uses HDR Histogram for accurate percentile calculations

### Limitations

- Single-threaded design (measures single-flow latency)
- UDP only (no TCP support)
- Loopback and local network optimized (WAN latency will be higher)

## License

MIT
