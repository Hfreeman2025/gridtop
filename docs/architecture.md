# Architecture

Gridtop is a synchronous Rust terminal application. Network acquisition runs off the
rendering thread; a cancellable channel keeps the countdown responsive while waiting.
There is no service, model runtime or browser dependency in the installed executable.

| Boundary                            | Responsibility                                                                                                         |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `api`, `model`, `atlas`             | Read-only HTTPS, validated timestamps, units, bands and complete regional coverage. Missing data is never zero-filled. |
| `schedule`, `carbon`                | Shared slot-aligned search, overlap-weighted integration, signed comparison and final launch checks.                   |
| `carbon_cli/args`                   | CLI parsing and planning constraints.                                                                                  |
| `carbon_cli/source`                 | Acquisition, dated recordings and the explicit unavailable-data fallback.                                              |
| `carbon_cli/mod`, `output`          | Waiting/launch orchestration and text/JSON output.                                                                     |
| `runner`, `signals`, `ledger`       | Foreground process group, signal forwarding and atomic per-run records without arguments or child output.              |
| `ui/`, `run_ui/`                    | Map, input, countdown rendering, forecast chart and terminal lifecycle. Drawing never launches a command.              |
| `build_support/raster`, `geography` | Even/odd polygon fill at build time, ID join and terminal resampling.                                                  |
| `capture`, `typography`             | Optional self-contained HTML export of terminal content and colours, with local Plex faces.                            |

The scheduler treats each forecast interval as constant. Integrating overlaps avoids
rounding a job’s expected runtime to a slot count; starting at API boundaries avoids
arbitrary-minute candidates when invocation happens mid-slot. The immediate comparison
uses the exact invocation instant. Earliest-start tie breaking makes the result deterministic.

Acquisition failure and planning failure are different contracts. Only acquisition failure
can enter the explicitly requested immediate fallback. Invalid deadlines, missing coverage,
recordings and stale or missed plans cannot quietly become permission to launch.

The ledger is written before execution starts. An initial write failure blocks launch;
a failure after the child finishes is reported without replacing its exit status. Each
record is atomically replaced and synced, allowing concurrent independent runs without
a shared append file or lost updates.

Tests inject clocks and forecast sources. Their synthetic jobs and state directories stay
inside temporary test folders. Public API recordings retain their dates and licences.
