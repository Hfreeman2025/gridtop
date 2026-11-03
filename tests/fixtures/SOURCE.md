# Public API recordings

Source: National Grid ESO / NESO Carbon Intensity API, https://api.carbonintensity.org.uk/.
Licence: CC BY 4.0, https://creativecommons.org/licenses/by/4.0/.
Downloaded with public, keyless, read-only GET requests on **1 November 2026**.
The API root page directly declares CC BY 4.0. No account, personal postcode or private data was used.

- `intensity.json`: `/intensity`, national observation for 10:00–10:30 UTC.
- `generation.json`: `/generation`, national generation mix for 10:00–10:30 UTC.
- `forecast.json`: `/intensity/{retrieval UTC datetime}/fw48h`, starts 10:30 UTC.
- `regional.json`: `/regional/intensity/{retrieval UTC datetime}/fw48h/regionid/13`, London.
- `postcode.json`: `/regional/postcode/SW1A`, the public outward code used by the brief's SW1 example.
- `provenance.json`: local file-save timestamps from the retrieval, preserved as a manifest.

Payloads are unmodified public API responses. Settlement times are authoritative; the save
manifest is provenance, not a substitute for those times. The application explicitly labels
replays as recordings and forbids their use to launch held commands. It uses the latest saved
snapshot timestamp as the recording's fetch/capture timestamp. Rendering and derived window
averages are gridtop's transformations; the data provider does not endorse the project.
