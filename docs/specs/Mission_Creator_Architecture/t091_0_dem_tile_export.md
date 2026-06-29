# T-091.0 — Everon DEM export + anchor verify

**Ticket:** T-091 · **Slice:** T-091.0  
**Status:** **shipped** @ `6d96339` (tag **T-091.0**)  
**Executor:** **claude-code** + enfusion-mcp  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

> **Shipped path:** `TBD_TerrainExportPlugin.c` resamples `WorldEditorAPI.GetTerrainSurfaceY` over a 6400² grid → 16-bit PNG. Manual WE **Export Height Map** is **dead** on packed Eden (terrain entity unselectable). Ops log: [`.ai/artifacts/t091_0_ops_log.txt`](../../../.ai/artifacts/t091_0_ops_log.txt).

---

## In one sentence

Commit Everon 16-bit DEM PNG + ≥10 engine-probed anchors under `packages/map-assets/everon/`; pass `make verify-terrain-strict`. **Tiles deferred** (T-090.1).

---

## Shipped artifacts (@ `6d96339`)

| Artifact | Path |
|----------|------|
| DEM PNG (LFS) | `packages/map-assets/everon/dem/everon-dem-16bit.png` — 6400×6400, 16-bit grayscale |
| Manifest | `packages/map-assets/everon/manifest.json` — `dem.source`: **`mod-getsurfacey-resample`** |
| Anchors | `packages/map-assets/everon/anchors/verification.json` (11 anchors) |
| Plugin | `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_TerrainExportPlugin.c` |
| Convert | `packages/tbd-schema/scripts/raw-u16-to-dem-png.mjs` |
| Verify | `packages/tbd-schema/scripts/lib/dem-sample.mjs` (pngjs `{ skipRescale: true }`) |

**Gate result:** `maxDeltaM = 0.204 m` (threshold 1.0), 11/11 anchors PASS.  
**DEM sha256:** `585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b`

---

## Locked decisions (as shipped)

| Decision | Choice |
|----------|--------|
| **DEM source** | **`mod-getsurfacey-resample`** — Workbench plugin loops `GetTerrainSurfaceY` |
| **Manual WE export** | **Dead** on vanilla Eden (packed-world lock); do not retry |
| **Spawn alignment** | Same heightfield as T-092 (`GetSurfaceY` authority) |
| **Encoding range** | Fixed V4: H_min **−204.78**, H_max **375.53** (not measured from sample extrema) |
| **Grid** | 6400×6400 @ 2 m (`metersPerPixel` 2); sample at pixel `(px,py)` → world `x = px/(W−1)×12800`, `z = py/(H−1)×12800` |
| **Base vs Modified A/B** | N/A — single resampled surface (no UI export) |
| **Tiles** | **Deferred** — not gated by `verify-terrain-strict` |
| **Re-export** | Plugins → TBD → **Export TBD Terrain DEM**; menuPath `"Plugins,TBD,Export TBD Terrain DEM"` |

---

## Re-export runbook (Everon refresh)

1. Open Eden (or sub-scene) in Workbench with **Net API** enabled.
2. **Plugins → TBD → Export TBD Terrain DEM** (or MCP `wb_execute_action` with menuPath above).
3. Plugin writes to `$profile:`:
   - `TBD_TerrainExport_heightmap.txt` (ASCII uint16 rows)
   - `TBD_TerrainExport_meta.json`
4. Convert:
   ```bash
   node packages/tbd-schema/scripts/raw-u16-to-dem-png.mjs \
     --raster "$PROFILE/TBD_TerrainExport_heightmap.txt" \
     --meta "$PROFILE/TBD_TerrainExport_meta.json" \
     --out packages/map-assets/everon/dem/everon-dem-16bit.png
   ```
5. Re-probe anchors if needed; run `make verify-terrain-strict`.

**Spike gate:** plugin runs a probe + 40k-sample benchmark before the full 41M loop (~2.5 min @ 6400²).

---

## Manual WE export (blocked — historical)

Vanilla `{853E92315D1D9EFE}worlds/Eden/Eden.ent`: `GenericTerrainEntity` grey/unselectable; **Terrain Tool** (`Ctrl+T`) disabled. Community + MCP confirm packed-world lock. **Do not use** as primary path.

Reference wiki (custom terrains only): [Terrain Creation Tool — Export Height Map](https://community.bohemia.net/wiki/Arma_Reforger:World_Editor:_Terrain_Creation_Tool)

---

## Optional: MCP anchor probes

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'
bash scripts/mod/mcp-call.sh wb_terrain '{"action":"getHeight","x":4839.2,"z":6620.8}'
```

Mandatory bridgehead coords — see **Anchor set** below.

---

## Anchor set (shipped — 11)

| id | x | z |
|----|---|---|
| `bridgehead-sl` | 4839.2 | 6620.8 |
| `bridgehead-tl0` | 4836.9 | 6626.5 |
| `bridgehead-tl1` | 4831.2 | 6628.8 |
| + `coast-sw`, `valley-inland`, `hill-north`, `coast-w`, `peak-central`, `seabed-e`, `shelf-ne`, `mid-s` | | |

Schema: [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json). Do **not** hand-fill `demYM` / `deltaM`.

---

## Mathematical verification contract (mandatory)

See prior sections V1–V7 in git history @ `6d96339`. Commands:

```bash
make verify-terrain
make verify-terrain-strict
make schema-validate
```

**pngjs note:** read 16-bit DEM with `{ skipRescale: true }`; use `.depth` not `.bitDepth` on pngjs objects.

---

## Out of scope (follow-on)

| Item | Ticket |
|------|--------|
| Frontend DEM loader | **T-091.1** — **shipped** @ `2c56c2e` |
| Z on place/move | **T-091.2** |
| Tile pyramid | **T-090.1** |
| Arland re-export / EMT / automation polish | **T-121** |

---

## Related

- [`t091_1_dem_loader.md`](t091_1_dem_loader.md)
- [`t121_terrain_dem_export_automation.md`](t121_terrain_dem_export_automation.md)
- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md)
