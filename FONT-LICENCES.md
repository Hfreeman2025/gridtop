# Font licences and provenance

IBM Plex latin WOFF2, **@fontsource 5.3.0**, **SIL Open Font License 1.1**.
Font licensing is separate from the Cargo dependency policy.
Upstream notices: [Sans](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans@5.3.0/LICENSE),
[Mono](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-mono@5.3.0/LICENSE),
[Sans Condensed](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans-condensed@5.3.0/LICENSE).

All eight files are self-hosted in `fonts/`, alongside [OFL.txt](fonts/OFL.txt), which preserves the
upstream copyright notice and complete licence for each of the three families. No font is modified.

| Family                  | Weight/style |  Bytes | Source                                                                                                                                           |
| ----------------------- | ------------ | -----: | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| ibm-plex-sans           | 400 normal   | 22,588 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans@5.3.0/files/ibm-plex-sans-latin-400-normal.woff2)                     |
| ibm-plex-sans           | 400 italic   | 24,356 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans@5.3.0/files/ibm-plex-sans-latin-400-italic.woff2)                     |
| ibm-plex-sans           | 500 normal   | 24,184 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans@5.3.0/files/ibm-plex-sans-latin-500-normal.woff2)                     |
| ibm-plex-sans           | 600 normal   | 24,252 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans@5.3.0/files/ibm-plex-sans-latin-600-normal.woff2)                     |
| ibm-plex-mono           | 400 normal   | 14,708 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-mono@5.3.0/files/ibm-plex-mono-latin-400-normal.woff2)                     |
| ibm-plex-mono           | 600 normal   | 15,620 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-mono@5.3.0/files/ibm-plex-mono-latin-600-normal.woff2)                     |
| ibm-plex-sans-condensed | 500 normal   | 19,952 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans-condensed@5.3.0/files/ibm-plex-sans-condensed-latin-500-normal.woff2) |
| ibm-plex-sans-condensed | 600 normal   | 19,816 | [@fontsource 5.3.0](https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans-condensed@5.3.0/files/ibm-plex-sans-condensed-latin-600-normal.woff2) |

Total WOFF2 download: **165,476 bytes**. Exact byte lengths and SHA-256 digests are recorded in
[fonts/SOURCES.json](fonts/SOURCES.json). Styles and weights are explicit in [fonts/plex.css](fonts/plex.css),
all with `font-display: swap`. No variable or synthetic bold font is used.

Capture HTML embeds these same local files as font data URLs and carries the full licence text,
so exported evidence opens offline under `file://`. The regular Mono, regular Sans and semibold
Condensed faces are preloaded. There are no runtime Google or CDN font requests and no font install.
The screenshots use semantic Plex roles inside the existing terminal cell geometry. Unicode block,
braille and box-drawing glyphs may use the host's monospace fallback where absent from the latin fonts.

The Rust TUI cannot select a terminal emulator's font or mix proportional and monospace families.
For native use, select **IBM Plex Mono regular 400 / bold 600** in your own terminal settings. WOFF2
assets here are for browser evidence; native font installation is a separate user action. Gridtop does not install fonts or change terminal settings.
