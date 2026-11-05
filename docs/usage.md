# Using gridtop

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
instant with `Z` or an explicit offset, for example `2030-11-03T18:00:00+00:00`. Bare local times are
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
Gridtop implements its own exact search in [schedule.rs](../src/schedule.rs) over the
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
the standard user-local state directory. Paths must be absolute. Tests and recordings override this into the
project's `.tmp/`; they never read or write a real home ledger. Records use private permissions and
one atomic, synced file per run, with queued, starting, running and terminal states. A failed initial
ledger write blocks launch. A post-run write error is reported without replacing the child's exit code.

Only the executable's basename, plan, timestamps, status, exit code and forecast estimates are stored.
**Arguments, environment, stdin and child output are never logged.** `runs` shows the last recorded
state; a hard kill or machine shutdown can leave an unfinished record. There is no background daemon,
resume service or unattended restart after a crash.

See [validation](validation.md) for measured results and [screenshots](images/runner-wide.png) for the recorded countdown.

## Controls

| Key / input                | Action                                                     |
| -------------------------- | ---------------------------------------------------------- |
| j/k, ↑/↓, mouse wheel      | Select a region                                            |
| ←/→, h/l                   | Inspect the preceding/next half-hour                       |
| Home / End                 | Follow the active interval / inspect the final interval    |
| p                          | Play/stop forecast frames, one real half-hour per second   |
| Click map or region row    | Select a region                                            |
| Click timeline             | Select a forecast time                                     |
| t, or click the theme name | Phosphor → amber → monochrome                              |
| m / v                      | Map / landscape                                            |
| Tab / Shift+Tab            | Move visible focus between map, regions and timeline       |
| Space                      | Pause/resume landscape motion                              |
| [ / ]                      | Cleanest window duration, 1–6 hours                        |
| r                          | Queue a refresh; the 60-second request floor still applies |
| ? / Esc                    | Open / close help                                          |
| q / Ctrl+C                 | Quit and restore terminal modes, cursor and mouse capture  |

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

The one-shot commands remain national by default. The map opens with London selected; that selected region is named in its detail and highlighted in the list. `--region 1..14` selects a mapped region. Aggregate regions 15–17 open their generation landscape; they also work with `now`, `best` and `wait`. Postcode lookup is public and keyless. `SW1` explicitly resolves to `SW1A` in London because the API rejects `SW1`; outward codes such as `RG10` are accepted. The original small one-shot demo recordings cover GB and London; the map recording covers all 14 regions.

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

All data times are UTC. Regional generation mix describes consumption including imports, not demand, capacity or physical flows. Colours are forecasts of intensity, not simulated weather. See [fixture provenance](../tests/fixtures/SOURCE.md), `atlas-provenance.json`, and [boundary provenance](../data/SOURCES.md).
