# Boundary provenance

Supported by National Energy SO Open Data.

Official NESO GB DNO Licence Areas, 20240503, EPSG:27700 (British National Grid).
Source: https://api.neso.energy/dataset/0e377f16-95e9-4c15-a1fc-49e06a39cfa0/resource/1c6a7dc0-1b6c-443a-bc67-5f7125649434/download/gb-dno-license-areas-20240503-as-geojson.geojson

The archive contains the unmodified `regions.geojson` and catalogue evidence from the
NESO download above, saved on 1 November 2026. Licence: NESO Open Data Licence, adapted from OGL v3.
`boundary-licence.html` retains the complete licence article; site navigation and active
content are excluded. Attribution does not imply NESO endorsement.

`build.rs` independently rasterises the original MultiPolygons, including holes and islands,
with an even/odd scanline fill. It writes a 384×768 region-ID raster into Cargo's OUT_DIR.
Runtime resampling preserves the projected aspect ratio. Small islands and London can be
sub-pixel at narrow sizes; numbered region markers and the keyboard list remain available.
The boundary map describes licence areas, not power flows or the transmission network.

Explicit DNO → Carbon Intensity API join (these IDs are different datasets):
17→1 North Scotland; 18→2 South Scotland; 16→3 North West; 15→4 North East;
23→5 Yorkshire; 13→6 North Wales/Merseyside; 21→7 South Wales; 14→8 West Midlands;
11→9 East Midlands; 10→10 East England; 22→11 South West; 20→12 South England;
12→13 London; 19→14 South East. Tests check public city coordinates, including London.
