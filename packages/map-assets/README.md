# map-assets

Per-terrain static assets for the Mission Creator basemap and DEM (T-090 / T-091).

| Path | In git | Purpose |
|------|--------|---------|
| `{terrain}/manifest.json` | yes | Terrain manifest — validates against `packages/tbd-schema/schema/terrain-manifest.schema.json` |
| `{terrain}/dem/*.png` | **Git LFS** | 16-bit elevation — Everon @ T-091.0 |
| `{terrain}/satellite/*.tbd-sat` | **Git LFS** | Unified satellite bundle (primary basemap @ T-090.1.2.8) |
| `{terrain}/tiles/` | **no** (local build) | WebP pyramid — optional fallback; rebuild locally |
| `{terrain}/anchors/verification.json` | yes | Engine-probed anchor log for `make verify-terrain-strict` |
| `{terrain}/staging/` | **no** (gitignored) | SAP ortho / export scratch |

| Terrain | Status |
|---------|--------|
| **Everon** | manifest + DEM + unified `everon-sat.tbd-sat` + 11 anchors; MC uses unified delivery first |
| **Arland** | stub `manifest.json` only (`widthPx/heightPx: 0`) — DEM deferred |

## Fresh clone

```bash
git lfs install
git lfs pull          # DEM + unified .tbd-sat
make map-assets-link  # symlink → frontend public/
```

**Tile pyramid missing?** Mission Creator still loads via the unified bundle. For pyramid fallback (or after `make map-water-everon`):

```bash
make map-water-everon   # Everon: restore → water composite → bundle + pyramid → verify
# or manually:
bash scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
```

`verify-tile-pyramid.mjs` **skips** when no pyramid is on disk (CI-safe).

See [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).
