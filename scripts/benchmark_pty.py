#!/usr/bin/env python3
"""Linux-only warm startup and idle samples. Measures a PTY, not display latency."""
import fcntl
import os
import pty
import select
import statistics
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/release/quickfox").resolve())
startup = []
with tempfile.TemporaryDirectory(prefix="quickfox-benchmark-") as root:
    for iteration in range(30):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        start = time.perf_counter()
        process = subprocess.Popen([binary, "--data-dir", root], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, TERM="xterm-256color"))
        output = b""
        deadline = start + 5
        while b"Your first 30-second practice" not in output:
            if time.perf_counter() > deadline:
                process.kill()
                raise RuntimeError("First frame was not observed within five seconds")
            if select.select([master], [], [], 0.1)[0]:
                output += os.read(master, 65536)
        startup.append((time.perf_counter() - start) * 1000)
        if iteration == 29:
            status = Path(f"/proc/{process.pid}/status").read_text()
            rss = next(line for line in status.splitlines() if line.startswith("VmRSS:"))
            def ticks():
                fields = Path(f"/proc/{process.pid}/stat").read_text().split()
                return int(fields[13]) + int(fields[14])
            before = ticks()
            time.sleep(1)
            idle = (ticks() - before) / os.sysconf("SC_CLK_TCK")
            print(rss)
            print(f"Idle CPU time in a 1-second menu sample: {idle:.3f} s")
            print(f"Initial terminal output: {len(output)} bytes (100 x 30)")
        os.write(master, b"q")
        process.wait(timeout=5)
        os.close(master)
        os.close(slave)
startup.sort()
print(f"Warm startup to first menu text, 30 PTY launches: median {statistics.median(startup):.2f} ms; p95 {startup[28]:.2f} ms; max {startup[-1]:.2f} ms")
