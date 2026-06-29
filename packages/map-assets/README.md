# map-assets

Per-terrain static assets for the Mission Creator basemap and DEM (T-090 / T-091).

| Path | Purpose |
|------|---------|
| `{terrain}/manifest.json` | Terrain manifest — validates against `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| `{terrain}/dem/everon-dem-16bit.png` | 16-bit elevation (Git LFS) — **Everon shipped** @ T-091.0 (`mod-getsurfacey-resample`) |
| `{terrain}/tiles/` | Aligned basemap tiles (Git LFS) — **T-090.1 / T-121** (not yet in repo) |
| `{terrain}/anchors/verification.json` | Engine-probed anchor log for `make verify-terrain-strict` |

**Everon:** manifest + DEM + 11 anchors populated @ `6d96339`. **Arland:** stub manifest only (`widthPx/heightPx: 0`).

See [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).
