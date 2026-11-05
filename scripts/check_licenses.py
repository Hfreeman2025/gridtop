#!/usr/bin/env python3
"""Check every resolved package, including inactive platforms. No network requests.
SPDX OR is a choice; AND requires every obligation to pass. Fail closed on unknowns.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

# ICU4X's icu_collections/icu_locale_core/icu_normalizer/icu_properties/icu_provider
# and their data crates use the permissive Unicode-3.0 licence required by patched IDNA.
ALLOWED = {
    "MIT",
    "Apache-2.0",
    "ISC",
    "BSD-1-Clause",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "Unicode-3.0",
}


def choose(expression):
    tokens = re.findall(r"\(|\)|[^\s()]+", expression.replace("/", " OR "))
    pos = 0

    def atom():
        nonlocal pos
        if pos >= len(tokens):
            raise ValueError("missing SPDX term")
        if tokens[pos] == "(":
            pos += 1
            value = either()
            if pos >= len(tokens) or tokens[pos] != ")":
                raise ValueError("unbalanced SPDX expression")
            pos += 1
            return value
        term = tokens[pos]
        pos += 1
        if pos < len(tokens) and tokens[pos] == "WITH":
            if pos + 1 >= len(tokens):
                raise ValueError("missing SPDX exception")
            term += " WITH " + tokens[pos + 1]
            pos += 2
        return [term] if term in ALLOWED else None

    def both():
        nonlocal pos
        value = atom()
        while pos < len(tokens) and tokens[pos] == "AND":
            pos += 1
            right = atom()
            value = (
                sorted(set(value + right))
                if value is not None and right is not None
                else None
            )
        return value

    def either():
        nonlocal pos
        value = both()
        while pos < len(tokens) and tokens[pos] == "OR":
            pos += 1
            right = both()
            if value is None:
                value = right
        return value

    value = either()
    if pos != len(tokens):
        raise ValueError("unrecognized SPDX syntax")
    return value


def main():
    # Ensure expression handling cannot accidentally admit an extra AND obligation.
    assert choose("Apache-2.0 AND ISC") == ["Apache-2.0", "ISC"]
    assert choose("(MIT OR Apache-2.0) AND Unicode-3.0") == ["MIT", "Unicode-3.0"]
    assert choose("(MIT OR Apache-2.0) AND GPL-3.0") is None
    assert choose("GPL-3.0 OR MIT") == ["MIT"]
    assert choose("Zlib OR Apache-2.0 OR MIT") == ["Apache-2.0"]
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
            text=True,
        )
    )
    rows = []
    failures = []
    for package in sorted(
        metadata["packages"], key=lambda p: (p["name"], p["version"])
    ):
        declared = package.get("license") or "UNKNOWN"
        selected = choose(declared)
        if selected is None:
            failures.append(f"{package['name']} {package['version']}: {declared}")
        rows.append(
            f"| {package['name']} | {package['version']} | {declared} | {' AND '.join(selected or ['REJECTED'])} |"
        )
    if failures:
        sys.exit("Disallowed dependency licences:\n" + "\n".join(failures))
    report = (
        """# Dependency licences

Generated from the locked Cargo metadata, including build and inactive platform dependencies.
An OR expression is used under the permitted alternative recorded below. Every term of an
AND expression must be permitted. Data from the Carbon Intensity API is separately CC BY 4.0. Boundary data is separately
under the NESO Open Data Licence; see data/SOURCES.md.

The local `unicode-ident` compatibility adapter is clean-room MIT code calling the
MIT/Apache-2.0 `unicode-xid` implementation. No upstream unicode-ident code or tables are copied.
`lru` and `unicode-normalization` remain constrained by the project licence policy.
Patched `url`/IDNA uses ICU4X (`icu_collections`, `icu_locale_core`, `icu_normalizer`,
`icu_properties`, `icu_provider` and their data crates) under permissive Unicode-3.0.
Keep the lockfile and run this check when upgrading.

| Package | Version | Declared SPDX | Selected licence(s) |
| --- | --- | --- | --- |
"""
        + "\n".join(rows)
        + "\n"
    )
    if "--write" in sys.argv:
        Path("DEPENDENCIES.md").write_text(report)
    else:
        if Path("DEPENDENCIES.md").read_text() != report:
            sys.exit(
                "DEPENDENCIES.md is out of date; run python3 scripts/check_licenses.py --write"
            )
    print(
        f"PASS: {len(rows)} packages; every selected licence is MIT, Apache-2.0, BSD, ISC or Unicode-3.0."
    )


if __name__ == "__main__":
    main()
