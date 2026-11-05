#!/usr/bin/env python3
"""Exercise the actual release TUI in owned pseudo terminals, with no network."""

import argparse
import re
import codecs
import fcntl
import json
import os
from pathlib import Path
import pty
import resource
import select
import signal
import struct
import subprocess
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/release/gridtop"
parser = argparse.ArgumentParser()
parser.add_argument("--output", default=".tmp/terminal-checks")
args = parser.parse_args()
OUTPUT = (ROOT / args.output).resolve()
assert OUTPUT.is_relative_to(ROOT / ".tmp")
OUTPUT.mkdir(parents=True, exist_ok=True)


def run_case(name, action, seconds=0):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 44, 120, 0, 0))
    before = termios.tcgetattr(slave)
    command = (
        [str(ROOT / "target/release/examples/terminal-panic")]
        if action == "panic"
        else [str(BIN), "--demo"]
    )
    if action == "reduced":
        command += ["--no-motion"]
    if action == "animate":
        command += ["--view", "landscape"]
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    process = subprocess.Popen(
        command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env={**os.environ, "TERM": "xterm-256color"},
        close_fds=True,
    )
    output = bytearray()
    cast_events = []
    decoder = codecs.getincrementaldecoder("utf-8")("strict")
    started = time.monotonic()
    ready_at = None
    sent = False
    quit_sent = False
    resize_offset = 0
    writes = 0
    while process.poll() is None:
        if time.monotonic() - started > 12:
            process.kill()
            process.wait()
            (OUTPUT / f"pty-failed-{name}.ansi").write_bytes(output)
            raise AssertionError(
                f"{name}: no expected terminal state; tail={output[-700:]!r}"
            )
        readable, _, _ = select.select([master], [], [], 0.01)
        if readable:
            chunk = os.read(master, 65536)
            output.extend(chunk)
            decoded = decoder.decode(chunk)
            if decoded:
                cast_events.append([round(time.monotonic() - started, 6), "o", decoded])
            writes += 1
        if ready_at is None and b"\x1b[?2026l" in output:
            ready_at = time.monotonic()
        if ready_at is not None and not sent and time.monotonic() - ready_at >= seconds:
            if action == "ctrlc":
                os.write(master, b"\x03")
            elif action == "sigterm":
                process.send_signal(signal.SIGTERM)
            elif action == "resize":
                resize_offset = len(output)
                # Keep help open during resize. An ESC immediately followed by a printable
                # byte is an Alt chord, not two independent human keypresses.
                os.write(master, b"?")
                fcntl.ioctl(
                    slave, termios.TIOCSWINSZ, struct.pack("HHHH", 28, 38, 0, 0)
                )
                process.send_signal(signal.SIGWINCH)
                # The next frame must contain the compact recording label before quitting.
                sent = True
                continue
            elif action != "panic":
                os.write(master, b"q")
            sent = True
        if (
            action == "resize"
            and sent
            and not quit_sent
            and b"Supported by National" in output[resize_offset:]
            and re.search(rb"\x1b\[28;[0-9]+H", output[resize_offset:])
        ):
            os.write(master, b"q")
            quit_sent = True
    # Drain final escape sequences after process termination.
    while select.select([master], [], [], 0)[0]:
        output.extend(os.read(master, 65536))
    wall = time.monotonic() - started
    after = termios.tcgetattr(slave)
    after_usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after_usage.ru_utime + after_usage.ru_stime - usage.ru_utime - usage.ru_stime
    # macOS may set PENDIN after tcsetattr: a kernel-managed pending-input bit,
    # not a raw-mode setting. Compare every other flag and control character.
    before[3] &= ~getattr(termios, "PENDIN", 0)
    after[3] &= ~getattr(termios, "PENDIN", 0)
    assert before == after, (
        f"{name}: terminal flags not restored: {before!r} != {after!r}"
    )
    assert b"\x1b[?1049l" in output, f"{name}: alternate screen not restored"
    assert b"\x1b[?25h" in output, f"{name}: cursor not restored"
    assert process.returncode == (101 if action == "panic" else 0), (
        name,
        process.returncode,
        output[-1000:],
    )
    (OUTPUT / f"pty-{name}.ansi").write_bytes(output)
    if name == "animated-6s":
        header = {
            "version": 2,
            "width": 120,
            "height": 44,
            "title": "gridtop · public recording, 1 Nov 2026",
            "env": {"TERM": "xterm-256color"},
        }
        lines = [json.dumps(header)] + [
            json.dumps(event, ensure_ascii=False) for event in cast_events
        ]
        (OUTPUT / "gridtop.cast").write_text("\n".join(lines) + "\n")
    os.close(master)
    os.close(slave)
    return {
        "case": name,
        "exit": process.returncode,
        "terminal_restored": True,
        "wall_seconds": round(wall, 3),
        "cpu_seconds": round(cpu, 4),
        "cpu_percent_one_core": round(cpu / wall * 100, 2),
        "output_batches": writes,
    }


results = [
    run_case("quit", "quit"),
    run_case("ctrl-c", "ctrlc"),
    run_case("sigterm", "sigterm"),
    run_case("resize", "resize"),
    run_case("panic", "panic"),
    run_case("animated-6s", "animate", 6),
    run_case("reduced-3s", "reduced", 3),
]
print(json.dumps(results, indent=2))
(OUTPUT / "terminal-checks.json").write_text(json.dumps(results, indent=2) + "\n")
