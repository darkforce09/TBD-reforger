# map-assets

Per-terrain static assets for the Mission Creator basemap and DEM (T-090 / T-091).

| Path | In git | Purpose |
|------|--------|---------|
| `{terrain}/manifest.json` | yes | Terrain manifest — validates against `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| `{terrain}/dem/*.png` | **Git LFS** (~72 MB Everon) | 16-bit elevation — Everon @ T-091.0 |
| `{terrain}/satellite/*.tbd-sat` | **Git LFS** (~153 MB Everon) | Unified satellite bundle (primary basemap @ T-090.1.2.8) |
| `{terrain}/tiles/` | **no** (local build) | WebP pyramid — optional fallback; rebuild locally |
| `{terrain}/anchors/verification.json` | yes | Engine-probed anchor log for `make verify-terrain-strict` |
| `{terrain}/staging/` | **no** (gitignored) | SAP ortho / export scratch |

| Terrain | Status |
|---------|--------|
| **Everon** | manifest + DEM + unified `everon-sat.tbd-sat` + 11 anchors; MC uses unified delivery first |
| **Arland** | stub `manifest.json` only (`widthPx/heightPx: 0`) — DEM deferred |

## Fresh clone / LFS

```bash
make lfs-dem   # ~72 MB — hillshade + map-engine tests
make lfs-sat   # ~153 MB — full satellite
# or: git lfs install && git lfs pull
```

**Serving (T-171):** Axum `GET /map-assets/*` (`MAP_ASSETS_DIR`, default from `apps/website/api/` CWD) ← Trunk proxy ← SPA same-origin fetch. No symlink step.

Full story: [`docs/website/DEV_RUNBOOK.md`](../../docs/website/DEV_RUNBOOK.md) §Map assets · conventions [`WHERE_DOES_X_GO.md`](../../docs/platform/WHERE_DOES_X_GO.md).

**Tile pyramid missing?** Mission Creator still loads Satellite via the unified bundle. For pyramid fallback (or Map view):

```bash
make map-water-everon
make map-cartographic-everon
```

`verify-tile-pyramid.mjs` **skips** when no pyramid is on disk (CI-safe).

See [`t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).
