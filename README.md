# Synapse

A bare-metal diagnostic tool that measures a system's true stimulus-response reflex time at the application level.

## What is Synapse? (Simple Explanation)

Synapse is a **high-precision stopwatch for measuring how fast two applications communicate**.

Think of it as an ultra-fast echo game:

- **Client** establishes a TCP connection and sends messages with sequence numbers
- **Server** immediately echoes each message back through the same connection
- **Measurement** starts when sent, stops when received

This "round-trip latency" is measured thousands of times for precision.

### Why It's Different

Most diagnostic tools measure **kernel-to-kernel** latency (like `ping` using ICMP, where the OS kernel responds directly). Synapse measures the **complete application-to-application journey**:

1. Client sends the packet (syscall + kernel)
2. Packet travels through the network
3. **Server application** processes it (kernel must wake the app and hand it the packet)
4. Packet returns to client (network + kernel + syscall)

This is crucial: the network may be fast, but the application might be slow due to GC pauses, scheduler overhead, or processing delays. Synapse captures the full application stack.

### Design Principles

- **TCP**: Reliable connection-oriented protocol for guaranteed delivery
- **Zero-allocation**: Server echoes packets without memory allocations
- **Blocking I/O**: Single-focused execution, no async runtime overhead

### The Verdict

Synapse answers: **"Can this system respond in under 1 millisecond?"**

- Mean < 1ms = ✓ PASS
- Mean ≥ 1ms = ✗ FAIL

## Getting Started

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
Synapse TCP server listening on 0.0.0.0:8080
Ready to accept connections and echo packets...
```

**Important:** Keep this terminal window open! The server needs to keep running. The server accepts multiple concurrent TCP connections, with each client handled in a separate thread.

### Step 6: Run the Client

Open a **second terminal window** (keep the server running in the first) and run the client:

```bash
cargo run --release --bin client -- --server 127.0.0.1:8080 --packets 500000
```

**Note:** The `--` separator is required to pass arguments to the client (not to Cargo).

The client will establish a TCP connection to the server, run the test over that persistent connection, and display results. You should see output showing progress, statistics, and a PASS/FAIL verdict.

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

#### Client Options

The client supports flexible configuration via CLI flags, with sensible defaults for all options:

- `--server <IP:PORT>`: Server address (default: `127.0.0.1:8080`)
- `--packets <N>`: Number of packets to send (default: `10000`)
- `--warmup <N>`: Number of warmup packets (default: `100000`)
- `--update <N>`: Dashboard update interval (default: `100`)
- `--timeout <ms>`: Socket timeout in milliseconds (default: `100`)
- `--quiet`: Disable terminal UI (progress bars, spinners) for non-interactive environments
- `--log-level <LEVEL>`: Log level - trace, debug, info, warn, error (default: `info`)
- `--log-format <FORMAT>`: Log format - text or json (default: `text`)

**Running with defaults** (no flags required):

```bash
cargo run --release --bin client
# Connects to 127.0.0.1:8080 and sends 10000 packets
```

**Examples:**

```bash
# Custom server and packet count
cargo run --release --bin client -- --server 192.168.1.10:9000 --packets 100000

# Production monitoring with JSON logging
cargo run --release --bin client -- --quiet --log-format json --packets 1000

# Development with debug logging
cargo run --release --bin client -- --log-level debug --packets 500

# CI/CD automated testing
cargo run --release --bin client -- --quiet --log-format json --log-level warn

# High-precision long-running test
cargo run --release --bin client -- --packets 1000000 --warmup 200000
```

#### Server Options

The server supports flexible configuration via CLI flags, with sensible defaults for all options:

- `--bind <ADDRESS>`: Bind address (default: `0.0.0.0`)
- `--port <PORT>`: Bind port (default: `8080`)
- `--update-interval <MS>`: Monitor update interval in milliseconds (default: `100`)
- `--quiet`: Disable terminal UI for non-interactive environments (Docker, systemd, etc.)
- `--log-level <LEVEL>`: Log level - trace, debug, info, warn, error (default: `info`)
- `--log-format <FORMAT>`: Log format - text or json (default: `text`)

**Running with defaults** (no flags required):

```bash
cargo run --release --bin server
# Binds to 0.0.0.0:8080 with text logging at info level
```

**Examples:**

```bash
# Custom bind address and port
cargo run --release --bin server -- --bind 192.168.1.10 --port 9000

# Production deployment with JSON logging
cargo run --release --bin server -- --quiet --log-format json --log-level warn

# Development with debug logging
cargo run --release --bin server -- --log-level debug

# Docker/container deployment
cargo run --release --bin server -- --quiet --log-format json

# Multi-environment setup
# Dev:     cargo run --release --bin server -- --bind 127.0.0.1 --port 8080
# Staging: cargo run --release --bin server -- --bind 0.0.0.0 --port 8081 --log-level debug
# Prod:    cargo run --release --bin server -- --quiet --log-format json --log-level info
```

**Note:** The release build uses aggressive optimizations (LTO, single codegen unit, panic abort) for maximum performance.

## Logging

Synapse uses structured logging for observability and debugging. Both client and server support:

- **CLI flags**: `--log-level` and `--log-format` for direct configuration
- **Environment variable**: `RUST_LOG` (takes precedence over CLI flags)

### Client Logging Examples

```bash
# Default: Info level with text format
cargo run --release --bin client

# Debug level via CLI flag
cargo run --release --bin client -- --log-level debug

# JSON format for log aggregation (Datadog, Splunk, ELK, etc.)
cargo run --release --bin client -- --log-format json

# Production setup: JSON logs with warn level
cargo run --release --bin client -- --log-level warn --log-format json

# Override via environment variable (takes precedence)
RUST_LOG=trace cargo run --release --bin client
```

### Server Logging Examples

```bash
# Default: Info level with text format
cargo run --release --bin server

# Debug level via CLI flag
cargo run --release --bin server -- --log-level debug

# JSON format for log aggregation (Datadog, Splunk, ELK, etc.)
cargo run --release --bin server -- --log-format json

# Production setup: JSON logs with warn level
cargo run --release --bin server -- --log-level warn --log-format json

# Override via environment variable (takes precedence)
RUST_LOG=trace cargo run --release --bin server
```

### Log Levels

- **`error`**: Critical errors that cause the application to fail
- **`warn`**: Warning conditions (e.g., connection errors, sequence mismatches, timeouts)
- **`info`**: Informational messages (default) - major phases and completion
- **`debug`**: Detailed debugging information (packet operations, socket events)
- **`trace`**: Very verbose tracing (most detailed)

### Example Output

With `RUST_LOG=debug`, you'll see detailed logs like:

```
2024-01-01T12:00:00.123Z INFO synapse: Starting Synapse client server=127.0.0.1:8080 packets=10000
2024-01-01T12:00:00.124Z DEBUG synapse::client::socket: Connecting TCP stream addr=127.0.0.1:8080
2024-01-01T12:00:00.125Z DEBUG synapse::client::socket: TCP stream connected successfully
2024-01-01T12:00:00.126Z INFO synapse: Starting warmup phase warmup_count=100000
2024-01-01T12:00:00.150Z DEBUG synapse::client::measurement: Packet received successfully latency_ns=12500 sequence=0
...
```

**Tip:** Use `RUST_LOG=debug` when troubleshooting connectivity issues or analyzing packet-level behavior.

## Development

This section covers development workflows for contributors and maintainers.

### Running Tests

Synapse includes both unit tests and integration tests. Run all tests with:

```bash
cargo test
```

**What's happening:** Cargo compiles the project in test mode and runs all tests in the `tests/` directory and test functions marked with `#[test]` in the source code.

**Run specific tests:**

```bash
# Run tests matching a pattern
cargo test test_end_to_end

# Run only unit tests (tests in src/)
cargo test --lib

# Run only integration tests (tests in tests/)
cargo test --tests

# Run a specific integration test file
cargo test --test client_test

# Run tests with output (show println! output)
cargo test -- --nocapture
```

**Test structure:**

- **Unit tests**: Located alongside source code in `src/` (e.g., `src/client/socket.rs` contains tests for socket functionality)
- **Integration tests**: Located in `tests/` directory
  - `tests/client_test.rs`: Client integration tests
  - `tests/integration/client_test.rs`: Additional integration scenarios

### Code Formatting

Format code according to Rust style guidelines:

```bash
# Check formatting without making changes
cargo fmt --check

# Format all code
cargo fmt
```

**Note:** The project uses `rustfmt` configured via `rust-toolchain.toml`. Formatting is enforced in CI/CD pipelines.

### Linting

Check code for common mistakes and style issues:

```bash
# Run Clippy linter
cargo clippy

# Run Clippy with stricter checks (all warnings as errors)
cargo clippy -- -D warnings

# Run Clippy for release builds (catches additional issues)
cargo clippy --release
```

**What Clippy checks:** Common Rust mistakes, performance issues, style inconsistencies, and suggestions for idiomatic Rust code.

**Note:** The project includes `clippy` in `rust-toolchain.toml` for consistent linting across environments.

### Code Coverage

Generate code coverage reports to identify untested code paths:

```bash
# Install cargo-tarpaulin (if not already installed)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# Generate coverage with output to terminal
cargo tarpaulin --out Stdout

# Exclude test files from coverage
cargo tarpaulin --exclude-files 'tests/*' --exclude-files 'src/bin/*'
```

**Coverage output:** HTML reports are generated in `tarpaulin-report.html` (open in a browser). Terminal output shows line-by-line coverage percentages.

**Alternative coverage tools:**

- `cargo-llvm-cov`: Uses LLVM's source-based coverage (requires nightly Rust)
- `grcov`: Works with both stable and nightly Rust

### Build Commands

**Debug build** (faster compilation, includes debug symbols):

```bash
cargo build
```

**Release build** (optimized, no debug symbols):

```bash
cargo build --release
```

**Build specific binaries:**

```bash
# Build only the server
cargo build --release --bin server

# Build only the client
cargo build --release --bin client
```

**Clean build artifacts:**

```bash
# Remove all build artifacts
cargo clean

# Remove only release artifacts
cargo clean --release
```

### Pre-Commit Checklist

Before committing code, ensure:

1. ✅ All tests pass: `cargo test`
2. ✅ Code is formatted: `cargo fmt`
3. ✅ No Clippy warnings: `cargo clippy -- -D warnings`
4. ✅ Code compiles: `cargo build --release`

**Quick validation command:**

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test && cargo build --release
```

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
# Increase TCP buffer sizes
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

### Real-Time Display During Test

While the test is running, you'll see a real-time display that updates every few hundred milliseconds, showing both performance metrics and a live visualization of packet flow through the OSI layers:

```
Warming up ✓ (1000/1000)

→ 0.039ms                CLIENT                    SERVER
Mean: 0.022ms            ┌──────────────────────┐  ┌──────────────────────┐
P99: 0.045ms             │ L7: APPLICATION      │  │ L7: APPLICATION      │
Rate: 31.1k pkt/s        ├──────────────────────┤  ├──────────────────────┤
                         │ L4: TRANSPORT        │  │ L4: TRANSPORT        │
                         ├──────────────────────┤  ├──────────────────────┤
                         │ L3: NETWORK          │  │ L3: NETWORK          │
                         ├──────────────────────┤  ├──────────────────────┤
                         │ L2: DATA LINK        │  │ L2: DATA LINK        │
                         ├──────────────────────┤  ├──────────────────────┤
                         │ L1: PHYSICAL         │  │ L1: PHYSICAL         │
                         └──────────────────────┘  └──────────────────────┘
                                            ──────────▶
████████████████████████████████  500000/500000  [00:00:16]
```

**What each element means:**

**Left Panel - Performance Metrics:**

- **Warming up**: Initial phase that prepares the system (populates ARP tables, warms CPU caches). The ✓ indicates completion.

- **→ (Current latency)**: The most recent packet's round-trip time. Color-coded:

  - Green: < 0.5ms (excellent)
  - Yellow: 0.5-1ms (acceptable)
  - Red: > 1ms (needs attention)

- **Mean**: Average latency across all packets measured so far. Color-coded green if < 1ms, red otherwise.

- **P99**: The 99th percentile latency—99% of packets are faster than this value. Useful for spotting outliers.

- **Rate**: Packets processed per second (k = thousands). Shows throughput of the measurement itself.

- **Progress bar**: Visual representation of test progress:
  - Filled blocks (█) show completed packets
  - Empty blocks (░) show remaining packets
  - Numbers show: `current/total [elapsed time]`

**Right Panel - OSI Layer Visualization:**

- **Educational visualization**: Animated representation of the packet journey through OSI layers, synchronized with actual packet transmission (not kernel-instrumented)
- **Layer highlighting**: When a layer is **bright**, the animation shows a packet traversing that layer
- **Color scheme**:
  - Layer 7 (Application): Blue → Bright blue when active
  - Layer 4 (Transport): Green → Bright green when active
  - Layer 3 (Network): Yellow → Bright yellow when active
  - Layer 2 (Data Link): Orange → Bright orange when active
  - Layer 1 (Physical): Red → Bright red when active
- **Animation flow**: Shows conceptual journey—client stack (descending) → network → server stack (ascending) → return path
- **Sampling**: Animation advances every 100th packet to remain human-perceivable at high throughput
- **Note**: Layers 5 (Session) and 6 (Presentation) are omitted because we use raw bytes for minimal protocol overhead

### Final Results Summary

After the test completes, you'll see a comprehensive summary:

```
┌─────────────────────────────┐
│  Synapse Results            │
└─────────────────────────────┘

Packets:  500000 sent, 0 lost (0.00%)
          └─ With TCP, packet loss should be 0% (TCP guarantees delivery, but timeouts can occur)

Duration: 16.44s
          └─ Test completed at 30.4k packets/second

Latency Statistics (round-trip time):
  Mean:      22.8 µs  ← Average latency
  Min:       10.2 µs  ← Fastest packet
  Max:     3016.8 µs  ← Slowest packet
  P50:       16.1 µs  ← 50% of packets are faster than this (median)
  P90:       40.1 µs  ← 90% of packets are faster than this
  P99:       45.3 µs  ← 99% of packets are faster than this
  P99.9:     82.8 µs  ← 99.9% of packets are faster than this

Latency Distribution (packet count by range):
      0-20 µs:  ██████████████████████████████  52.8% (264,000 packets)
     20-40 µs:  ████████████████                31.4% (157,000 packets)
     40-60 µs:  ████████                        14.2% ( 71,000 packets)
     60-80 µs:  ▌                                1.4% (  7,000 packets)
    80-100 µs:  ▌                                0.2% (  1,000 packets)
       >10 ms:  ▌                               <0.1% (      1 packets) ← MAX: 3.0ms

✓ PASS: Mean latency (0.023ms) is below 1ms threshold
```

**Understanding the Distribution:**

The bucket distribution visualization uses:

- **Bar length**: Relative to the largest bucket (longest bar = most packets)
- **Color coding**: Green (>50%), Cyan (>10%), White (<10%), Red (outliers >10ms)
- **Verdict**: ✓ PASS if Mean < 1ms, ✗ FAIL if Mean ≥ 1ms

**Performance Factors:** System load, CPU frequency scaling, scheduler preemption, memory pressure, and OS-level tuning (see recommendations above) can all affect latency.

## Technical Details

### What Does Synapse Measure?

Synapse measures **application-level round-trip latency** over a persistent TCP connection: `Client App → Client Kernel → Server Kernel → Server App → Server Kernel → Client Kernel → Client App`

This captures the full application stack, including processing overhead, scheduler latency, memory allocation, and application-level pauses—unlike network-level tools that only measure kernel-to-kernel connectivity. Each client establishes its own TCP connection, and the server handles multiple concurrent connections simultaneously.

### OSI Model Layers

Synapse's code operates at **Layer 7 (Application)** and **Layer 4 (Transport/TCP)**, bypassing Layers 5-6 since we use raw bytes for minimal overhead.

However, the **latency measurement captures the complete journey** through all active OSI layers (7→4→3→2→1→network→1→2→3→4→7). The OS kernel handles Layers 3-1, but their processing time is included in the round-trip measurement.

This means Synapse measures full application-to-application latency, including all application processing overhead and the entire network stack.

### Measurement Methodology

1. **Connection establishment**: Client establishes a TCP connection to the server (one-time overhead)
2. **Warm-up phase**: Populates ARP tables and warms CPU/OS caches over the persistent connection
3. **Measurement phase**: Uses `Instant::now()` before send and after receive
4. **Post-processing**: HDR Histogram for accurate percentile calculations
5. **Blocking I/O**: Uses `std::net::TcpStream` instead of async runtimes for minimal overhead and deterministic timing
6. **Connection reuse**: All packets in a test session use the same TCP connection, amortizing the initial handshake cost

### Message Format

Minimal TCP protocol (8 bytes per message):

- **Client → Server**: 8-byte sequence number (u64, little-endian: 0, 1, 2, ...) sent over a persistent TCP connection
- **Server → Client**: Echo response (same 8 bytes) sent back through the same connection

The client validates the echoed sequence matches. Zero serialization overhead, no parsing, zero-allocation hot path. All messages in a test session are sent over a single TCP connection, which is established once at the beginning and reused for all packets.

### Limitations

- Single connection per client (measures single-flow latency per connection)
- TCP only (connection-oriented protocol)
- Server handles multiple concurrent connections (one thread per connection)
- Loopback and local network optimized (WAN latency will be higher)

## License

MIT
