# Local measurements

Measured on 2026-11-16 for v0.2.0. These are observations on one machine, not portability or
end-to-end input-latency guarantees.

## Environment

- Intel Core i9-12900H, 20 logical CPUs exposed to WSL2.
- Ubuntu 24.04.4 LTS, Linux 6.6.87.2-microsoft-standard-WSL2, x86_64.
- rustc 1.99.1 with the committed Cargo.lock. Rust 1.88 also passed all-target checks.
- Release profile: size optimization, thin LTO, one codegen unit, stripped symbols.
- Python Unix pseudo-terminal, 100×30 cells, `TERM=xterm-256color`; inherited `NO_COLOR`
  selected monochrome output for the startup sample. No graphical terminal emulator
  or display pipeline was measured.

## Results

| Measurement | Observed result | Scope |
|---|---:|---|
| Native executable | 1,355,816 bytes (1.29 MiB) | `target/release/quickfox` |
| Menu resident memory | 3,040 KiB (2.97 MiB) | One `/proc/PID/status` sample |
| Idle menu CPU | 0.000 CPU seconds | One wall-clock second; kernel tick resolution |
| Warm startup median | 1.08 ms | 30 launches, process creation to first menu-item text |
| Warm startup p95 | 4.10 ms | Sorted sample 29 of 30 |
| Warm startup maximum | 9.52 ms | Same warm-cache launches |
| Startup output observed | 1,141 bytes | Bytes collected by the first-menu readiness check |
| Input + metrics p50 | 118 ns | 20,000 in-process samples, at most 200 retained characters |
| Input + metrics p95 | 161 ns | Same engine-only microbenchmark |
| Input + metrics p99 | 192 ns | Same engine-only microbenchmark |

The 30 launches share one temporary profile; the first also creates its configuration.
The startup test stops when the first menu item is observed, not when every pixel is
visible. The output count depends on read boundaries and is not a complete frame-size
benchmark. The engine measurement excludes event decoding, rendering, filesystem I/O,
and input/display hardware. Its small prompts do not characterize every possible
long-running word session. Memory and CPU samples are exploratory, not a stress test.

The local executable dynamically links glibc and libgcc_s; its highest required GLIBC
symbol version is **2.39**. Use a compatible Linux system, or rebuild on the target.
The archive contains documentation and captures, so its exact size is reported by the
packaging command rather than treated as a stable binary metric.

## Reproduce

```bash
cargo build --release --locked
cargo run --release --locked --example benchmark
python3 scripts/benchmark_pty.py
stat -c '%n %s bytes' target/release/quickfox
bash scripts/package-linux.sh
stat -c '%n %s bytes' target/packages/*.tar.gz
ldd target/release/quickfox
readelf --version-info target/release/quickfox
```

Run benchmarks on an otherwise idle machine and report the terminal, color environment,
compiler, operating system, CPU, and profile with results. Cold-cache startup, playing
RSS/CPU, per-frame output, and physical input-to-display latency remain unmeasured.
