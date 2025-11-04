# Synapse

A bare-metal diagnostic tool that measures a system's true stimulus-response reflex time at the application level.

## What is Synapse? (Simple Explanation)

Synapse is a **high-precision stopwatch for measuring how fast two applications communicate**.

Think of it as an ultra-fast echo game:

- **Client** shouts "HELLO!" with a sequence number
- **Server** immediately bounces it back
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

- **UDP**: Minimal overhead, no connection ceremony
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
Synapse server listening on 0.0.0.0:8080
Ready to echo packets...
```

**Important:** Keep this terminal window open! The server needs to keep running.

### Step 6: Run the Client

Open a **second terminal window** (keep the server running in the first) and run the client:

```bash
cargo run --release --bin client -- --server 127.0.0.1:8080 --packets 500000
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

#### Client Options

- `--server <IP:PORT>`: Server address (default: `127.0.0.1:8080`)
- `--packets <N>`: Number of packets to send (default: `10000`)
- `--warmup <N>`: Number of warmup packets (default: `200`)
- `--update <N>`: Dashboard update interval (default: `100`)
- `--timeout <ms>`: Socket timeout in milliseconds (default: `100`)

**Note:** The release build uses aggressive optimizations (LTO, single codegen unit, panic abort) for maximum performance.

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

### Real-Time Display During Test

While the test is running, you'll see a real-time display that updates every few hundred milliseconds, showing both performance metrics and a live visualization of packet flow through the OSI layers:

```
Warming up ✓ (200/200)

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
- **Note**: Layers 5 (Session) and 6 (Presentation) are omitted because UDP is connectionless and uses raw bytes

### Final Results Summary

After the test completes, you'll see a comprehensive summary:

```
┌─────────────────────────────┐
│  Synapse Results            │
└─────────────────────────────┘

Packets:  500000 sent, 0 lost (0.00%)
          └─ Packet loss should be 0% for reliable measurements

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

Synapse measures **application-level round-trip latency**: `Client App → Client Kernel → Server Kernel → Server App → Server Kernel → Client Kernel → Client App`

This captures the full application stack, including processing overhead, scheduler latency, memory allocation, and application-level pauses—unlike network-level tools that only measure kernel-to-kernel connectivity.

### OSI Model Layers

Synapse's code operates at **Layer 7 (Application)** and **Layer 4 (Transport/UDP)**, bypassing Layers 5-6 since UDP is connectionless and uses raw bytes.

However, the **latency measurement captures the complete journey** through all active OSI layers (7→4→3→2→1→network→1→2→3→4→7). The OS kernel handles Layers 3-1, but their processing time is included in the round-trip measurement.

This means Synapse measures full application-to-application latency, including all application processing overhead and the entire network stack.

### Measurement Methodology

1. **Warm-up phase**: Populates ARP tables and warms CPU/OS caches
2. **Measurement phase**: Uses `Instant::now()` before send and after receive
3. **Post-processing**: HDR Histogram for accurate percentile calculations
4. **Blocking I/O**: Uses `std::net` instead of async runtimes for minimal overhead and deterministic timing

### Message Format

Minimal UDP protocol (8 bytes per packet):

- **Client → Server**: 8-byte sequence number (u64, little-endian: 0, 1, 2, ...)
- **Server → Client**: Echo response (same 8 bytes)

The client validates the echoed sequence matches. Zero serialization overhead, no parsing, zero-allocation hot path.

### Limitations

- Single-threaded design (measures single-flow latency)
- UDP only (no TCP support)
- Loopback and local network optimized (WAN latency will be higher)

## License

MIT
