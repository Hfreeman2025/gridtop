# Validation

The application targets macOS and Linux with Rust 1.94.0 or newer. Local results are
reported separately from GitHub Actions, which has not been dispatched from this checkout.

## Reproduce

```sh
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 scripts/check_licenses.py
python3 scripts/check_fonts.py
cargo build --release --locked --examples --bin gridtop
python3 scripts/pty_check.py
python3 scripts/runner-native.py --lifecycle-only
cargo run --release --locked --example runner-bench
cargo run --release --locked --example render-bench
```

All application tests use public saved forecasts or synthetic sources. They do not need
an API key, a local model, a live forecast or a user’s ledger. One top-level ignored test
is a subprocess-only orchestration helper invoked by integration and pseudo-terminal checks.

The licence check includes all locked Rust packages, including inactive target platforms.
`cargo audit` checks the lockfile against the public RustSec advisory database and requires
network access to refresh that database. CI treats that as a required check. [Known upstream warnings](../SECURITY.md) remain visible;
the URL/IDNA vulnerability is patched and no advisories are ignored.

Install the pinned advisory checker separately when running the audit locally:

```sh
cargo install cargo-audit --version 0.22.2 --locked
cargo audit
```

## Browser exports

Browser tooling needs Node.js 20 or newer and is separate from the Rust application. Install it
locally, then generate the same frames used by the native renderer:

```sh
npm ci
npm run format:check
npm audit --audit-level=high
npx playwright install chromium webkit firefox
cargo run --locked --example runner-gallery
cargo run --locked --example map-gallery
cargo run --locked --example theme-gallery
npm run check:browser -- chromium
npm run check:browser -- webkit
npm run check:browser -- firefox
```

Run one browser engine at a time. The harness refuses to launch while another automated
engine is present. It blocks external requests and live media, uses an offline audio context,
and keeps fresh profiles and reports in ignored `.tmp/` folders. No user browser profile is read.

The 81 cases include loading, errors, unavailable and empty data, retained forecasts,
long commands/region names, cancelled and dry-run states, help, all three themes and
164×50, 80×24 and 44×48 layouts. Each browser checks overflow, console errors/warnings,
all three Plex families, and cell boxes during a forced fallback-to-Plex swap. Chromium’s
PerformanceObserver supplies CLS; an unavailable API is reported as unavailable rather than zero.

Capture pages contain no JavaScript. The harness injects its probes only for verification.
These checks establish export rendering; the separate pseudo-terminal cases establish native
input, signals and cleanup.

## Measurements

The scheduling oracle covers **300 cases** using an independent minute-resolution search.
It includes half-hour starts, partial end slots, mid-slot invocation and earliest-start ties.
The Rust suite currently has **73 passing tests** and one subprocess-only ignored entry.

Native frame benchmarks draw **1,000 frames per case** through ratatui’s TestBackend in a
release build. Runner cases cover three themes at three sizes. This measures CPU preparation
of terminal cells, not terminal-emulator paint or a display’s frame rate. Timing varies with
machine load. Measured locally on **5 November 2026**, macOS ARM64, Rust 1.94.0, on a shared machine:

| Runner size | Frames per theme | Worst p95 across three themes |
| ----------- | ---------------: | ----------------------------: |
| 164×50      |            1,000 |                      0.422 ms |
| 80×24       |            1,000 |                      0.146 ms |
| 44×48       |            1,000 |                      0.133 ms |

Parsing the bundled regional recording took **2.342 ms**; choosing the exact plan took
**0.029 ms** in this run. There is no model download. These are observed local timings,
not a promise of latency on another machine or a measurement of the live API.

The refactor also passed **18 native PTY cases** (7 monitor cases and 11 runner cases),
including three dry-run recordings and eight controlling-terminal lifecycle cases. An
independent comparison of all **81 regenerated exports** against the previous version
found identical terminal text, glyphs and foreground/background colours.

## Browser results

The final local serial pass on **5 November 2026** passed **243 cases**, with 81 exports
in each engine. All eight font faces loaded; every export had zero overflow, console
errors or warnings, external requests and changed cell boxes during the forced font swap.
The inspected screenshots include wide, 80×24 monochrome and phone layouts.

| Engine   | Version       | Cases | CLS             | Font-swap geometry changes |
| -------- | ------------- | ----: | --------------- | -------------------------: |
| Chromium | 153.0.8010.12 |    81 | 0               |                          0 |
| Webkit   | 26.6          |    81 | API unavailable |                          0 |
| Firefox  | 155.0         |    81 | API unavailable |                          0 |

WebKit and Firefox do not expose the layout-shift observer in this environment. Their
geometry comparisons passed; no CLS value is claimed for either engine. All three engines
closed before the shared browser slot was released.

## Development tooling licences

These tools are separate from the installed Rust executable. Versions are fixed by
`package-lock.json`, the workflow and the commands below.

| Tool                         | Version | Licence          | Source                                  |
| ---------------------------- | ------- | ---------------- | --------------------------------------- |
| Playwright / playwright-core | 1.63.0  | Apache-2.0       | https://github.com/microsoft/playwright |
| Prettier                     | 3.6.2   | MIT              | https://github.com/prettier/prettier    |
| Ruff                         | 0.14.3  | MIT              | https://github.com/astral-sh/ruff       |
| cargo-audit                  | 0.22.2  | Apache-2.0 / MIT | https://github.com/RustSec/rustsec      |

Python helpers use only the standard library at runtime. To apply their formatter:

```sh
python3 -m pip install ruff==0.14.3
ruff format scripts
ruff check scripts
```

A source-only checkout passed `cargo check --locked --offline --all-targets` with the
available dependency cache, verifying that the shipped source, fixtures, fonts and boundary
data provide the required build inputs.
