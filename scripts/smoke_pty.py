#!/usr/bin/env python3
"""Linux/macOS PTY smoke tests. Uses only Python's standard library."""
import fcntl
import json
import os
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/quickfox").resolve())

class Game:
    def __init__(self, root, *arguments, environment_overrides=None):
        self.master, self.slave = pty.openpty()
        self.size(100, 30)
        self.before = termios.tcgetattr(self.slave)
        environment = dict(os.environ, TERM="xterm-256color", COLORTERM="truecolor")
        environment.pop("NO_COLOR", None)
        environment.update(environment_overrides or {})
        self.process = subprocess.Popen([binary, "--data-dir", str(root), *arguments], stdin=self.slave, stdout=self.slave, stderr=self.slave, env=environment)
        self.output = b""
        self.drain(0.25)
        assert self.process.poll() is None, self.output.decode(errors="replace")
    @property
    def plain(self):
        return b" ".join(re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b" ", self.output).split())
    @property
    def screen(self):
        # Reconstruct the small ANSI subset Ratatui emits. Raw output is differential,
        # so searching concatenated writes cannot reliably assert the visible text.
        grid = [[" "] * self.columns for _ in range(self.rows)]
        x = y = 0
        for part in re.split(r"(\x1b\[[0-?]*[ -/]*[@-~])", self.output.decode(errors="replace")):
            if part.startswith("\x1b["):
                command, parameters = part[-1], part[2:-1]
                if parameters.startswith("?"):
                    continue
                values = [int(v) if v.isdigit() else 0 for v in parameters.split(";")]
                amount = values[0] or 1
                if command in ("H", "f"):
                    y = (values[0] or 1) - 1
                    x = ((values[1] if len(values) > 1 else 1) or 1) - 1
                elif command == "G":
                    x = amount - 1
                elif command == "C":
                    x += amount
                elif command == "D":
                    x = max(0, x - amount)
                elif command == "A":
                    y = max(0, y - amount)
                elif command == "B":
                    y += amount
                elif command == "J" and values[0] == 2:
                    grid = [[" "] * self.columns for _ in range(self.rows)]
                elif command == "K" and 0 <= y < self.rows:
                    for column in range(max(0, x), self.columns):
                        grid[y][column] = " "
            else:
                for character in part:
                    if character == "\r":
                        x = 0
                    elif character == "\n":
                        y += 1
                    elif character.isprintable():
                        if 0 <= x < self.columns and 0 <= y < self.rows:
                            grid[y][x] = character
                        x += 1
        return "\n".join("".join(row) for row in grid)
    def size(self, columns, rows):
        self.columns, self.rows = columns, rows
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        if hasattr(self, "process"):
            import signal
            self.process.send_signal(signal.SIGWINCH)
    def drain(self, duration=0.15):
        until = time.monotonic() + duration
        while time.monotonic() < until:
            ready, _, _ = select.select([self.master], [], [], max(0, until - time.monotonic()))
            if ready:
                self.output += os.read(self.master, 65536)
    def send(self, data, delay=0.15):
        os.write(self.master, data)
        self.drain(delay)
    def finish(self):
        deadline = time.monotonic() + 5
        while self.process.poll() is None and time.monotonic() < deadline:
            self.drain(0.05)
        self.process.wait(timeout=1)
        self.drain(0.05)
        assert self.process.returncode == 0, self.output.decode(errors="replace")
        assert termios.tcgetattr(self.slave) == self.before, "Terminal modes were not restored"
        assert b"\x1b[?1049l" in self.output, "Alternate screen was not restored"
        assert b"\x1b[?2004l" in self.output, "Bracketed paste was not disabled"
        os.close(self.master)
        os.close(self.slave)

with tempfile.TemporaryDirectory(prefix="quickfox-smoke-") as directory:
    root = Path(directory)
    game = Game(root / "home")
    game.send(b"\t")
    assert "Workspace" in game.screen, game.screen
    assert "Commands  /" not in game.screen, game.screen
    game.send(b"\x10")
    assert "Search commands" in game.screen, game.screen
    game.send(b"\x1b")
    assert "Workspace" in game.screen, game.screen
    game.send(b"\x1b")
    assert "Your first 30-second practice" in game.screen, game.screen
    game.send(b"q")
    game.finish()
    print("PASS Tab menu and Ctrl+P commands are distinct; Esc and exit restore context")

    game = Game(root / "interrupt", "campaign")
    game.send(b"abc")
    game.send(b"\x03")
    game.finish()
    assert not (root / "interrupt/data/progress.json").exists()
    print("PASS Ctrl+C restores terminal; abandoned sessions do not save scores")

    game = Game(root / "resize", "campaign")
    game.send(b"abc")
    game.size(40, 10)
    game.drain(0.3)
    assert b"Resize to at least" in game.plain
    game.size(100, 30)
    game.drain(0.3)
    assert b"The clock is stopped" in game.plain
    game.send(b"\r")
    game.send(b"\x1b")
    game.send(b"q")
    game.send(b"q")
    game.finish()
    print("PASS resize pauses, recovery resumes, and leaving returns home")

    game = Game(root / "complete", "play", "--words", "25")
    game.send(b"\x1b[200~pasted content\x1b[201~")
    assert b"Paste ignored" in game.plain
    game.send(b"x" * 800)
    game.drain(0.4)
    saved = json.loads((root / "complete/data/progress.json").read_text())
    assert saved["sessions"] == 1
    assert len(saved["history"]) == 1
    assert saved["completed_missions"] == 0
    assert b"NET WPM" in game.plain
    game.send(b"\x03")
    game.finish()
    exported = subprocess.check_output([binary, "--data-dir", str(root / "complete"), "export"])
    assert json.loads(exported)["sessions"] == 1
    print("PASS paste blocked, word session completes once, progress persists and exports")

    game = Game(root / "navigation", "campaign")
    game.send(b"abc")
    game.send(b"\x07")  # Ctrl+G keeps the session paused at Home.
    assert b"Resume session" in game.plain
    game.send(b"\x10")  # Ctrl+P opens commands.
    game.send(b"settings")
    game.send(b"\r")
    assert b"Settings" in game.plain
    config_path = root / "navigation/config/config.json"
    settings = json.loads(config_path.read_text())
    settings["theme"] = "ember"
    config_path.write_text(json.dumps(settings))
    game.send(b"\x1b[15~")  # F5 reloads the external edit.
    assert b"JSON configuration reloaded" in game.plain
    assert b"Ember" in game.plain
    config_path.write_text("{unfinished edit")
    game.send(b"\x1b[15~")
    assert "Config kept unchanged" in game.screen, game.screen
    game.send(b"\x1b[C")  # A UI preference change must not overwrite the invalid edit.
    assert config_path.read_text() == "{unfinished edit"
    config_path.write_text(json.dumps(settings))
    game.send(b"\x1b[15~")
    game.send(b"\x07")
    game.send(b"\r")
    game.send(b"\x03")
    game.finish()
    assert json.loads(config_path.read_text())["theme"] == "ember"
    assert not (root / "navigation/data/progress.json").exists()
    print("PASS menu resume, command search, live JSON reload, and external-edit protection")

    sound_root = root / "sound-fallback"
    (sound_root / "config").mkdir(parents=True)
    (sound_root / "config/config.json").write_text(json.dumps({"sound": {"enabled": True}}))
    game = Game(sound_root, environment_overrides={"PULSE_SERVER": "unix:/tmp/quickfox-no-audio-server"})
    game.send(b"\t")
    for _ in range(6):
        game.send(b"\x1b[B", 0.02)
    game.send(b"\r")
    game.send(b"2")
    game.drain(0.3)
    assert "Audio unavailable" in game.screen, game.screen
    game.send(b"\x1b[C")
    assert "Sound is off" in game.screen, game.screen
    assert not json.loads((sound_root / "config/config.json").read_text())["sound"]["enabled"]
    game.send(b"\x10")
    game.send(b"quick")
    game.send(b"\r")
    game.send(b"\r")
    game.send(b"abc")
    game.send(b"\x03")
    game.finish()
    print("PASS unavailable audio stays responsive, can be muted, and does not block typing")

    game = Game(root / "daily", "daily")
    for _ in range(60):
        game.send(b"x" * 100, 0.02)
    daily_save = root / "daily/data/progress.json"
    deadline = time.monotonic() + 10
    while not daily_save.exists() and time.monotonic() < deadline and game.process.poll() is None:
        game.drain(0.1)
    assert daily_save.exists(), game.screen
    saved = json.loads(daily_save.read_text())
    assert saved["sessions"] == 1 and len(saved["daily_best"]) == 1
    assert saved["history"][0]["daily_day"] is not None
    assert saved["completed_missions"] == 0
    game.send(b"\x03")
    game.finish()
    print("PASS daily challenge completes and stores a separate local best")

    result = subprocess.run([binary], capture_output=True)
    assert result.returncode != 0 and b"interactive terminal" in result.stderr
    print("PASS non-TTY startup fails with an actionable message")
