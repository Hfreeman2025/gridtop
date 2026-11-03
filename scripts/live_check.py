#!/usr/bin/env python3
"""Optional read-only public-API smoke checks. Not part of general tests or CI.
The only child side effect possible is a marker inside target/, which must never appear.
"""
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/release/gridtop"
MARKER = ROOT / "target/held-command-must-not-run"
assert not MARKER.exists(), "remove the unexpected marker after investigating it"

def held_case(name, args, expected):
    process = subprocess.Popen([str(BIN), *args, "--", "touch", str(MARKER)],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output = bytearray()
    started = time.monotonic()
    while expected not in output:
        if process.poll() is not None or time.monotonic() - started > 25:
            process.kill()
            raise AssertionError((name, bytes(output)))
        if select.select([process.stderr], [], [], 0.1)[0]:
            output.extend(os.read(process.stderr.fileno(), 65536))
    process.send_signal(signal.SIGINT)
    stdout, stderr = process.communicate(timeout=20)
    output.extend(stderr)
    assert process.returncode == 130, (name, process.returncode, bytes(output))
    assert not MARKER.exists(), (name, "held command ran")
    assert not stdout
    (ROOT / "shots" / f"live-{name}.txt").write_bytes(output)
    return {"case": name, "exit": process.returncode, "command_started": False,
            "observed": expected.decode()}

results = [held_case("cancel", ["wait", "--below", "1"], b"waiting for < 1"),
           held_case("offline", ["--postcode", "ZZ99", "wait", "--below", "2000"], b"OFFLINE")]
# Prove child exit propagation end-to-end with an immediate, achievable live threshold.
child = subprocess.run([str(BIN), "wait", "--below", "2000", "--", "false"], capture_output=True)
assert child.returncode == 1 and b"Condition met" in child.stderr
results.append({"case": "child-exit", "exit": child.returncode, "command_started": True})
print(json.dumps(results, indent=2))
(ROOT / "shots/live-checks.json").write_text(json.dumps(results, indent=2) + "\n")
