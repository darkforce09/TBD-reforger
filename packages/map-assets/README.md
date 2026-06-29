# map-assets

Per-terrain static assets for the Mission Creator basemap and DEM (T-090 / T-091).

| Path | Purpose |
|------|---------|
| `{terrain}/manifest.json` | Terrain manifest — validates against `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| `{terrain}/dem/*.png` | 16-bit elevation (Git LFS) — Everon only @ T-091.0 |
| `{terrain}/tiles/` | Aligned basemap tiles (Git LFS) — **T-090.1 / T-121** (not yet in repo) |
| `{terrain}/anchors/verification.json` | Engine-probed anchor log for `make verify-terrain-strict` |

| Terrain | Status |
|---------|--------|
| **Everon** | manifest + DEM (6400², 71,911,548 bytes) + 11 anchors @ `6d96339` |
| **Arland** | stub `manifest.json` only (`widthPx/heightPx: 0`) — DEM deferred |

See [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).
