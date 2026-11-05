#!/usr/bin/env python3
"""Validate the vendored fonts against their source manifest, without a network request."""

import hashlib
import json
from pathlib import Path

records = json.loads(Path("fonts/SOURCES.json").read_text())
assert len(records) == 8
for record in records:
    data = (Path("fonts") / record["file"]).read_bytes()
    assert data[:4] == b"wOF2"
    assert len(data) == record["bytes"]
    assert hashlib.sha256(data).hexdigest() == record["sha256"]
    assert record["version"] == "5.3.0" and record["licence"] == "SIL OFL 1.1"
    assert record["source"].startswith("https://cdn.jsdelivr.net/npm/@fontsource/")
css = Path("fonts/plex.css").read_text()
assert css.count("@font-face") == 8
assert css.count("font-display: swap") == 8
assert Path("fonts/OFL.txt").read_text().count("SIL OPEN FONT LICENSE Version 1.1") == 3
print("PASS: 8 pinned local fonts, swap declarations and complete family licences.")
