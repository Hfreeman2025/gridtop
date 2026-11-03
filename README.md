# gridtop

gridtop schedules a local command for the cleanest complete window in a regional electricity forecast, then passes through its input, output, signals and exit code. Its terminal countdown shows the exact plan and the intensity difference from starting immediately. A private local ledger keeps forecast comparisons; grams appear only when you supply an energy estimate. The original fourteen-region GB map and generation landscape are still here. No account, key, daemon or hosted service.

![Real recorded regional forecasts, 1 November 2026](shots/v3/pass-2/preview-wide.png)

## See it in ten seconds

Requires macOS or Linux, Rust 1.94.0+, a C compiler, and a Unicode terminal. From this folder:

```sh
./target/release/gridtop run --demo --dry-run --watch --for 2h --watts 800 -- cargo test --release
```

The local executable is already built. This preview uses the genuine public forecast saved on **1 November 2026** and a clearly labelled **hypothetical** 800 W load. It computes the plan using the real scheduler and replays its clock; it never starts the command or writes a ledger. **Ctrl-C exits.** For a fresh checkout, build first with `cargo build --release --locked`. A personal install is `cargo install --path . --locked`; nothing is installed globally by the build or tests.

Run `./target/release/gridtop --demo` for the original map. There, **→** changes the forecast time, **j/k** selects a region, **t** cycles the themes, **v** opens the landscape and **m** returns to the map. Without `--demo`, the interactive views connect to the live API. A dated preview is explicit while loading; `--live-only` keeps the screen neutral until live data arrives. The runner's countdown changes only as time passes or a real forecast refreshes; there is no decorative animation.

```sh
cargo build --release --locked
export PATH="$PWD/target/release:$PATH"
gridtop
gridtop --theme mono
gridtop --view landscape --no-motion
gridtop --postcode SW1
gridtop --region 13
```

Startup previews are confined to interactive views. Scripting output requires live data or an explicit, permitted recording mode. **Recordings can never release jobs.**

The map is rasterised from official NESO DNO boundaries **at build time**, including holes and islands. A 384×768 region-ID mask is embedded in the executable. Runtime scaling preserves British National Grid proportions and uses terminal half-blocks; no geometry service or browser is needed. The source DNO IDs differ from Carbon Intensity IDs: the explicit join and public city-coordinate tests prevent a London polygon from receiving another region’s forecast. Numbered markers locate regions; they do not represent power stations. There are no drawn power flows.

A comfortable terminal is 164×50. The 44×48 layout keeps all 14 regions visible on a phone-sized terminal. Smaller terminals scroll the regional list around the selection; below 28×22 the map gives a clear size notice. `now --json` works without an interactive terminal.

The v2.1 plots use filled eighth-block bars. At 80×24, three regional rows stay visible with the scroll hint in the header. [Monochrome at 80×24](shots/v2.1/pass-2/mono-compact.png) and [all checks](DONE-V2.1.md).

## Carbon-aware runner

```sh
gridtop run --region 13 --within 12h --for 2h --watts 800 -- cargo test --release
gridtop run --postcode SW1 --within 24h --for 45m --dry-run -- python3 -m compileall .
gridtop next --region 13 --within 12h --for 2h --json
gridtop runs
gridtop runs --json
```

`run` and `next` default to **London (13)**, **one hour** of expected runtime, within **24 hours**.
All fourteen mapped regions and public outward postcode lookup work. The map's old scripting commands
remain national by default. Use either `--within` (up to 48h) or `--deadline`. A deadline is an RFC3339
instant with `Z` or an explicit offset, for example `2026-11-03T18:00:00+00:00`. Bare local times are
rejected: the repeated hour at the autumn DST change must not silently pick the wrong instant. Output
is UTC. A distant deadline is bounded by the available 48h forecast.

The whole expected runtime must fit. Starts are API half-hour boundaries at or after planning time;
ends may be mid-slot when `--for` is not a multiple of 30 minutes. The start is fixed once chosen.
While waiting, the screen stays responsive and refreshes at most once per minute. The displayed
forecast updates while its original immediate-start baseline is retained. The final launch gate
checks freshness, area, coverage, lateness and the deadline again. It refuses a start missed by a
minute, or an expected finish beyond the deadline. Allow scheduling slack for network and OS latency.

`--for` is an **expected runtime**, not a timeout: the runner does not kill a job when it reaches that
length. The comparison assumes constant load over that duration. Ctrl-C while waiting cancels without
spawning. During execution the child gets the foreground terminal and its original argument array;
there is no shell interpolation. Its stdin/stdout/stderr pass through. SIGINT, SIGTERM, SIGHUP, SIGQUIT, SIGUSR1 and SIGUSR2 sent to gridtop are
forwarded to the child's process group, including grandchildren. Keyboard Ctrl-C goes directly to the
foreground child, avoiding duplicate delivery. Foreground ownership and original terminal settings
are restored afterwards. The child's exit code passes through, including `128 + signal`. This is not a full shell job-control
manager: a Ctrl-Z/fg/bg suspension workflow is not supported.

`--dry-run` prints a plan, with `--json` for a machine-readable one; it never starts a command or
writes a ledger. `--watch` adds the interactive dry-run countdown. `--demo` is allowed only with
`--dry-run` on `run`; it is also available for `next`. A recording remains marked throughout.

A failed fetch, invalid forecast or failed launch gate is an explicit error. **Only
`--now-if-unavailable` permits immediate execution after an acquisition failure.** This opt-in never
turns a recording, impossible schedule or invalid deadline into permission to run. An immediate
fallback has **no intensity or emissions estimate**. There is no silent fallback.

### Method and estimates

The technique is **temporal shifting**, also used by the
[Green Software Foundation Carbon Aware SDK](https://github.com/Green-Software-Foundation/carbon-aware-sdk).
Gridtop implements its own exact search in [schedule.rs](src/schedule.rs) over the
[official regional Carbon Intensity API](https://api.carbonintensity.org.uk/).
For each feasible API slot start it integrates the piecewise-constant forecast over the complete
expected runtime. The minimum wins; ties choose the earliest start. This is exhaustive optimization
on the supplied forecast, not a learned predictor or a promise about future generation.

```text
average(t) = sum(forecast_i × seconds_overlapping_slot_i) / expected_runtime_seconds
avoided intensity = average(start immediately) - average(chosen window)
estimated grams = avoided intensity × supplied or derived kWh
kWh from watts = supplied watts × expected_runtime_seconds / 3,600,000
```

The immediate-start comparison begins at the exact planning instant, including partial edge slots.
A negative difference stays negative: delaying can be forecast to increase intensity. Energy and
intensity are separate inputs, as in the operational-emissions term of the
[Software Carbon Intensity methodology](https://sci.greensoftware.foundation/). This tool does **not**
claim a full SCI score: it does not estimate embodied emissions or a functional unit.

`--kwh` supplies expected total energy; `--watts` assumes constant power over `--for`. Both are explicit
assumptions. Neither is measured by gridtop. **No energy input means no grams.** The ledger compares
its initial baseline with the launch-time forecast; revisions can affect that estimate. A child that
finishes early, runs longer or fails does not magically acquire a measured energy reading. Actual
runtime and exit status are recorded separately. No historical grid verification or measured savings
are claimed. This compares forecast average consumption intensity, not marginal dispatch effects or
causally verified avoided emissions. The independent minute-resolution oracle covers 300 scheduling cases.

### Local run ledger

The state directory is `$GRIDTOP_STATE_DIR`, else `$XDG_STATE_HOME/gridtop`, else
`$HOME/.local/state/gridtop`. Paths must be absolute. Tests and recordings override this into the
project's `.tmp/`; they never read or write a real home ledger. Records use private permissions and
one atomic, synced file per run, with queued, starting, running and terminal states. A failed initial
ledger write blocks launch. A post-run write error is reported without replacing the child's exit code.

Only the executable's basename, plan, timestamps, status, exit code and forecast estimates are stored.
**Arguments, environment, stdin and child output are never logged.** `runs` shows the last recorded
state; a hard kill or machine shutdown can leave an unfinished record. There is no background daemon,
resume service or unattended restart after a crash.

The new screenshots, native demo recordings, scores and performance are in
[DONE-V3.md](DONE-V3.md) and `shots/v3/`. The original map's evidence stays in `shots/v2/` and `shots/v2.1/`.

## Controls

| Key / input | Action |
| --- | --- |
| j/k, ↑/↓, mouse wheel | Select a region |
| ←/→, h/l | Inspect the preceding/next half-hour |
| Home / End | Follow the active interval / inspect the final interval |
| p | Play/stop forecast frames, one real half-hour per second |
| Click map or region row | Select a region |
| Click timeline | Select a forecast time |
| t, or click the theme name | Phosphor → amber → monochrome |
| m / v | Map / landscape |
| Tab / Shift+Tab | Move visible focus between map, regions and timeline |
| Space | Pause/resume landscape motion |
| [ / ] | Cleanest window duration, 1–6 hours |
| r | Queue a refresh; the 60-second request floor still applies |
| ? / Esc | Open / close help |
| q / Ctrl+C | Quit and restore terminal modes, cursor and mouse capture |

`--no-motion` or `GRIDTOP_REDUCED_MOTION=1` freezes the landscape and disables automatic forecast playback. `NO_COLOR` selects monochrome in the TUI. There is no flashing or idle map animation. Terminal frames use synchronized-update sequences where supported and ordinary terminal diffing elsewhere. Unknown terminals simply ignore the synchronization hint.

The map, numbers and selected-region detail use the **same forecast interval**. The bars run from the first to last API interval and share a vertical scale. Each column preserves the highest forecast of its underlying half-hour slots and uses that slot’s API band colour. Reversed blocks overlap each region’s cleanest window (including windows narrower than a column); this marker also works in monochrome. The triangle above the bars marks the selected half-hour. A horizontal baseline means a true zero; missing data stays blank. The map colours use the API’s intensity **bands**, which are supplied by the provider. In monochrome, the selected-region band is also written in words, and every region has a numeric reading.

## Scripts and scheduling

```sh
gridtop now
gridtop now --json
gridtop best --within 12h --duration 2h
gridtop best --within 12h --duration 90m --json
gridtop --region 13 now --json
gridtop wait --below 100 -- python3 -c 'print("condition met")'
gridtop wait --cleanest-within 12h -- python3 -c 'print("forecast window reached")'
```

The one-shot commands remain national by default. The map opens with London selected; that selected region is named in its detail and highlighted in the list. `--region 1..14` selects a mapped region. Aggregate regions 15–17 open their generation landscape; they also work with `now`, `best` and `wait`. Postcode lookup is public and keyless. `SW1` explicitly resolves to `SW1A` in London because the API rejects `SW1`; outward codes such as `RG10` are accepted. The original small one-shot demo recordings cover GB and London; the v2 map recording covers all 14 regions.

Durations are whole minutes or hours, 1 minute to 48 hours. Every best-window start is an **API half-hour boundary at or after now**. The full requested duration must fit inside both the horizon and available forecast. A 45-minute duration can end mid-slot; a two-hour duration starts and ends on half-hours. The lowest time-weighted average wins; ties choose the earliest start. If no complete aligned window fits, the command errors instead of moving the horizon. CLI, TUI and cleanest-wait share this implementation. A 300-case independent oracle checks it.

`wait --below` uses the active interval’s forecast, **strictly less** than the threshold, with data fetched less than five minutes ago. `wait --cleanest-within` selects one complete **30-minute** window, counts down, and launches at its start with current data available. Offline or stale data holds the command. Expiry errors without launching. A recording cannot release a command. Interrupting a held command exits 130. Keep the process running: this is a local gate, not a durable scheduler.

The child is spawned through `std::process::Command` with a literal argument array, never an interpolated shell. Its exit code is returned, including `128 + signal` on Unix. Forecasts are estimates; gridtop does not promise emissions savings.

## Data, provenance and failure states

All API requests are read-only HTTPS to `api.carbonintensity.org.uk`, with rustls, OS trust roots, no redirects or proxies, a 5-second connection timeout, a 15-second request timeout, and a 2 MiB body limit. The map uses one `/regional/intensity/{UTC datetime}/fw48h` request per minute. Postcode/aggregate selection also uses the relevant reading endpoint. The original CLI uses `/intensity`, `/intensity/{from}/fw48h`, `/generation` and regional/postcode endpoints. Normal builds never fetch data.

The all-region parser requires all 14 mapped regions in all 96 contiguous half-hours, unique known IDs, consistent names, valid timestamps and intensity bands, plausible values, all nine unique fuels, finite 0–100 percentages, and generation totals within rounding tolerance. It never silently fills a missing region with zero. Extra aggregate IDs are recognised and excluded from the map.

- **Connecting:** a genuine, dated recording keeps the engine visible and says RECORDING … connecting. It never says LIVE DATA before a validated live response.
- **Live-only / slow:** `--live-only` keeps the map neutral and values `--` until data arrives; elapsed waiting time is explicit.
- **Empty / API error:** no invented values; the error is visible and the map stays neutral.
- **Failed refresh:** the last successful forecast remains, with an OFFLINE label and its saved date.
- **Old data:** STALE is explicit. An intentionally inspected future interval stays labelled FORECAST.
- **Recording:** its real saved date stays visible; no date shifting or automatic demo fallback.

All data times are UTC. Regional generation mix describes consumption including imports, not demand, capacity or physical flows. Colours are forecasts of intensity, not simulated weather. See [fixture provenance](tests/fixtures/SOURCE.md), `atlas-provenance.json`, and [boundary provenance](data/SOURCES.md).

## Checks and evidence

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo fetch --locked
python3 scripts/check_licenses.py
cargo build --release --locked --examples --bin gridtop
python3 scripts/pty_check.py
cargo run --release --locked --example render-bench
cargo run --release --locked --example v2-gallery -- pass-2
cargo run --release --locked --example runner-gallery -- pass-2
python3 scripts/runner-native.py
cargo run --release --locked --example runner-bench
```

CI checks Linux and macOS without live API calls or publishing. It is authored locally; remote CI has not been dispatched. Development builds disable incremental debug artifacts to keep rebuildable cache size modest.

`shots/v2/` contains two visual critique passes at **1440×900** and **390×844**, an actual terminal recording, lifecycle checks, browser reports and native rendering measurements. [CRITIQUE.md](shots/v2/CRITIQUE.md) records defects found and changes made. The screenshots render exact ratatui cells through local HTML; they are not a separate web implementation. Actual interaction and cleanup are checked independently with `TestBackend`, mouse/keyboard events and native pseudo terminals.

The optional screenshot harness uses only the installed Playwright package explicitly permitted by the brief. Browser downloads may be kept under `.tmp/browsers`; no application code from another project is read or reused:

```sh
node scripts/v2-screenshots.mjs chromium pass-2
PLAYWRIGHT_BROWSERS_PATH="$PWD/.tmp/browsers" node scripts/v2-screenshots.mjs webkit pass-2
PLAYWRIGHT_BROWSERS_PATH="$PWD/.tmp/browsers" node scripts/v2-screenshots.mjs firefox pass-2
```

The capture pages ship **zero JavaScript**. Their browser checks establish correct evidence rendering, not native TUI compatibility with browsers. `og.png` is a 1200×630 capture of the real runner using the dated public forecast. `favicon.svg` is generated from the same boundary raster. Nothing is served or published. Full results, performance numbers and remaining weaknesses are in [DONE-V2.md](DONE-V2.md).

Reproduce the v2.1 theme/size captures and actual native recordings:

```sh
cargo run --locked --example v2_1-gallery -- pass-2
node scripts/v2_1-screenshots.mjs chromium pass-2
python3 scripts/v2-tour.py --output shots/v2.1/wide
python3 scripts/v2-tour.py --output shots/v2.1/compact --width 80 --height 24
```

The new evidence and two-pass critique are in `shots/v2.1/`; the original v2 evidence stays dated and separate.

Short excerpt from the actual recorded terminal:

```text
gridtop / GB regional carbon intensity
RECORDING · 01 Nov 2026
FORECAST  01 Nov 15:30–16:00 UTC

13 London             245 gCO₂/kWh · very high
Wind 5.8%   Solar 3.6%   Gas 57.7%   Imports 15.6%
Cleanest 2h  Mon 04:00–06:00 UTC  49 g/kWh
```

## Credits and licences

Personal project by Harry Freeman. Code: [MIT](LICENSE), Copyright (c) 2026 Harry Freeman.

Electricity data: **National Grid ESO / NESO Carbon Intensity API**, [api.carbonintensity.org.uk](https://api.carbonintensity.org.uk/), **[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)**. The API root declares that licence. Recorded responses are unmodified; charts, window averages and map colouring are gridtop’s transformations, without provider endorsement.

Boundaries: **Supported by National Energy SO Open Data.** Official NESO GB DNO Licence Areas 20240503, EPSG:27700, under the **NESO Open Data Licence** adapted from OGL v3. Original data, licence and catalogue evidence are retained in `data/`. Boundary data was the only material read from the permitted grid-weather folder. The source DNO-to-API ID join, rasterisation, rendering and interaction code were independently written for gridtop.

All 196 resolved packages, including build and inactive platform dependencies, pass the MIT / Apache-2.0 / BSD / ISC allowlist. [DEPENDENCIES.md](DEPENDENCIES.md) records every selected alternative and all AND obligations. `url`, `lru` and `unicode-normalization` are constrained for that allowlist. The local clean-room MIT `unicode-ident` adapter delegates to MIT/Apache `unicode-xid`; no upstream tables with an additional Unicode licence are copied. Electricity and boundary data licences are separate from dependency licences.

Local commits require explicit authorization. No remote, pushes, publishing, accounts or global installation. A multi-hour real-time wait and remote Linux CI remain unrun; scheduler clocks and live short paths have been checked separately.
