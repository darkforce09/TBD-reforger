# map-assets

Per-terrain static assets for the Mission Creator basemap and DEM (T-090 / T-091).

| Path | Purpose |
|------|---------|
| `{terrain}/manifest.json` | Terrain manifest — validates against `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| `{terrain}/dem.png` | 16-bit elevation (Git LFS) — added at T-091.0 export |
| `{terrain}/tiles/` | Aligned basemap tiles (Git LFS) — added at T-090.1 |

**Stub manifests** use `widthPx/heightPx: 0` until Workbench export. See [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).
