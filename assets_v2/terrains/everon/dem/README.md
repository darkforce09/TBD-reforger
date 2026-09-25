# Everon elevation model

Everon's ground height as one 16-bit greyscale image, 2 m per pixel. The map engine turns it into
the hillshade, the contour lines, the sea band and the height readout of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map, and the tools and gates
sample it for spot heights, water and the anchor check.

## Contents

```text
assets_v2/terrains/everon/dem/
└── everon-dem-16bit.png  the 6400 × 6400 16-bit height grid of the whole island, 2 m per pixel
```

## Format

- Encoding: a 16-bit single-channel PNG, 6400 × 6400 samples covering the manifest's
  `worldBounds` (0 to 12,800 m on both axes), with column 0 at x = 0 and row 0 at z = 0, the south
  edge: the exporter writes row `py` at world z = `py` × step, and the map engine samples row 0 at
  the bounds' minimum z, since the manifest's `axisFlip` is false on both axes. A sample `v` is
  `min + v / 65535 · (max − min)` metres, with `min` −204.78 m and `max` 375.53 m, so black is the
  deepest seabed and white the highest peak. About 72 MB, stored in Git LFS
  (`.gitattributes`: `assets_v2/terrains/**/*.png`); a clone without LFS content holds a pointer
  file here.
- Schema: the manifest's `dem` block describes the file (`path`, `widthPx`, `heightPx`,
  `encoding` `uint16-linear`, the height range, `source` `mod-getsurfacey-resample` and
  `axisFlip`), following `contracts_v2/definitions/terrain-manifest.schema.json`. The decoder is
  `decode_png_to_meters` in `apps/website/map-engine/src/world/terrain/dem/`.
- Adding a file: a re-export replaces the image and the manifest's `dem` block together, then
  `cargo xtask schema terrain-alignment --terrain everon --strict` checks it against the anchors.

## Producers and consumers

- Producers: the `tbd-export` DEM plugin in [Workbench](/documentation_v2/glossary.md#workbench)
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/`) samples the engine's
  surface height into a raw grid and its metadata; `cargo run -p developer-tools --bin world --
  raw-u16-dem-png --raster <grid> --meta <meta> --out <png>` packs them into this image.
- Consumers:
  - the map engine's elevation loader, which fetches `/map-assets/everon/dem/everon-dem-16bit.png`
    at boot (`apps/website/map-engine/src/world/terrain/dem/`), and its spot-height and relief
    code;
  - the map raster pipeline's height-label export and inland water analysis
    (`tools_v2/developer-tools/src/map_raster_pipeline/`);
  - `cargo xtask schema terrain-alignment`, which samples it at every anchor, and
    `cargo xtask schema height-labels`;
  - `cargo xtask ci lfs-dem` and the `map-engine` and `schema` jobs of
    `.github/workflows/ci.yml`, which pull only this LFS object, and the map engine's spot-height
    tests and the world export's elevation tests, which read it from disk.

## Boundaries

- Depends on: the `dem` block of `assets_v2/terrains/everon/manifest.json`, which must name this
  file with its true size and height range.
- Used by: the map engine, the map raster pipeline, the anchor gate, CI and the tests listed above.
- Rules: the image and the manifest's `dem` block change together; every anchor in
  `assets_v2/terrains/everon/anchors/verification.json` stays within 1 m of the image
  (`cargo xtask schema terrain-alignment --terrain everon --strict`).

## Related documentation

- [Elevation model](/apps/website/map-engine/src/world/terrain/dem/README.md) — decoding, sampling
  and the vector grid.
