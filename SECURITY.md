# Security and dependency maintenance

Gridtop launches local programs only after an explicit command-line request. It preserves
argument boundaries, forwards signals to the child process group and never interprets
arguments through a shell. Recorded forecasts cannot release a job. The run ledger excludes
arguments, environment, stdin and child output.

Live acquisition is restricted to the fixed ASCII HTTPS origin
`api.carbonintensity.org.uk`. Redirects and environment proxies are disabled; postcodes,
times and region IDs are validated. Responses have bounded size and timeouts. This is
an input boundary, not a claim that dependency advisories can be ignored.

## Audit policy

CI runs `cargo audit` against the public RustSec database and fails on vulnerabilities.
There are **no audit ignore entries**. The lockfile uses patched `url` 2.5.8 and
IDNA 1.1.0, addressing
[RUSTSEC-2024-0421 / CVE-2024-12224](https://rustsec.org/advisories/RUSTSEC-2024-0421.html).
The required ICU4X libraries use the permissive Unicode-3.0 licence, recorded by the
[dependency licence gate](DEPENDENCIES.md).

The current ratatui 0.29 dependency graph still reports these upstream warnings:

| Dependency   | Advisory                                                                   | Status                                   |
| ------------ | -------------------------------------------------------------------------- | ---------------------------------------- |
| lru 0.12.4   | [RUSTSEC-2026-0002](https://rustsec.org/advisories/RUSTSEC-2026-0002.html) | Mutable-iterator soundness warning.      |
| lru 0.12.4   | [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html) | Panic-safety soundness warning in `pop`. |
| paste 1.0.15 | [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) | Upstream is unmaintained.                |

These warnings remain visible in audit output. They are not represented as patched or
suppressed. Updating ratatui and its cache/macro dependencies is a separate maintenance
item, requiring the rendering and terminal-lifecycle checks to be rerun. An audit exit
status of zero does not mean there are no upstream warnings.

When reporting a problem, include the gridtop version, operating system, relevant flags
and a minimal synthetic reproduction. Do not attach a real run ledger, credentials,
private commands or child output to a public issue.
