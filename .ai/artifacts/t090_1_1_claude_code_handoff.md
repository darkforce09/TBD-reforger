# T-090.1.1 — Claude Code handoff (Map cartographic view)

**Slice:** T-090.1.1 · **Executor:** claude-code · **Branch:** `main` (single stream)  
**Spec (authority):** [`docs/specs/Mission_Creator_Architecture/t090_1_1_map_cartographic_view.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_1_map_cartographic_view.md)  
**Cross-cutting UX:** [`t090_basemap_dual_view.md`](../../docs/specs/Mission_Creator_Architecture/t090_basemap_dual_view.md)

---

## What you are building

Enable the **Map** basemap tab in Mission Creator — stylized cartographic tiles (roads + terrain palette, **no** aerial photo) — as a full **WebP pyramid** plus UI switch. Satellite stays untouched.

```text
P0 source spike (G1-A..D) → staging ortho → tiles/map pyramid
  → make map-cartographic-everon → manifest patch
  → frontend: basemapView map branch + enable Map radio
  → verify M1–M9 → tag T-090.1.1
```

---

## Critical product distinction

| Tab | Accept | Reject |
|-----|--------|--------|
| **Satellite** | SAP / photographic ortho | MapDataExporter stylized raster |
| **Map** | MapDataExporter / engine stylized / N9 synth | SAP supertexture paste |

T-090.1.2.4 rejected stylized capture for **Satellite** — that same capture tier is **the Map view product** @ T-090.1.1.

---

## Bootstrap already on `main` (do not redo)

| Area | State |
|------|-------|
| Manifest `tiles.map` stub | `packages/map-assets/everon/manifest.json` — no tiles on disk yet |
| Satellite pyramid + unified | Shipped through T-090.1.2.8 — **frozen** |
| `.topo` decoder | `scripts/map-assets/decode-topo.mjs` — vector input for synth path |
| Water classifier | T-090.1.2.5.2 — optional blue-water tint on Map ortho |
| `basemapView` store | Coerces `'map'`→`'satellite'` until you ship — **remove coercion** |
| Map radio | Disabled in `MissionSettingsDialog.tsx` — **enable** |
| Basemap layer | `useTerrainBasemapLayer.ts` — satellite-only — **add map branch** |

**Baseline:** `./scripts/ticket brief T-090 --slice T-090.1.1` · `make schema-validate` exit 0.

---

## P0 spike (do first — honest-stop)

Write **`.ai/artifacts/t090_1_1_source_spike.json`** with winner + rejects.

Try in order (stop at first shippable full-world ortho):

1. **G1-A** — Re-run / upscale **`MapDataExporter.ExportRasterization`** output (4096² from T-090.1 ship — check staging if TGA still present). Warp to 12800×12800.
2. **G1-B** — Decode **`defSatMap_BCR.edds`** or terrain default map background from pak (`decode-edds.mjs`).
3. **G1-D** — Workbench GUI export if offline paths fail (log human step; still ship pyramid from captured TGA).
4. **G1-C** — **N9 synth:** DEM hillshade + land-cover LUT + rasterized `.topo` roads + water mask — manifest `source: synthesized-cartographic`.

**Do not** block ship waiting for Workbench plugin automation — that's **T-090.3**.

---

## Execution order (strict)

1. **P0** — spike JSON + one 12800² (or documented upscale) north-up PNG in `staging/map/`.
2. **P1** — `build-tile-pyramid.sh` → `packages/map-assets/everon/tiles/map/`.
3. **P2** — `build-map-cartographic.mjs` + **`make map-cartographic-everon`** + verify script `VIEW=map`.
4. **P3** — Manifest `tiles.map.source` + encoding fields.
5. **P4** — Frontend: `basemapView.ts`, `MissionSettingsDialog.tsx`, `useTerrainBasemapLayer.ts` map branch.
6. **P5** — `.ai/artifacts/t090_1_1_verify_log.md` (M1–M9) + manual H2 contact note.
7. Tag **`T-090.1.1`** · prefix **`T-090.1.1:`**

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Edit `docs/**`, `.ai/tickets/registry.json`, CLAUDE status | Cursor doc sync after ship |
| Touch satellite bundle / `map-water-everon` / SAP ortho | Frozen @ T-090.1.2.5.2 |
| Put stylized map into unified satellite bundle | Wrong tab |
| Disable Map radio after ship | N9 requires Map to load (even if labeled Synthetic) |
| Re-open water heuristic tuning | T-143 / operator good-enough |

---

## Preflight

```bash
# repo root, main
git status   # clean or only your slice work
make schema-validate
./scripts/ticket prompt T-090 --slice T-090.1.1
node scripts/map-assets/decode-topo.mjs --terrain everon --stats
ls packages/map-assets/everon/tiles/map 2>/dev/null || echo "no map tiles yet (expected)"
```

---

## Verify commands

```bash
make map-cartographic-everon          # after you add the target
VIEW=map node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make schema-validate
make verify-terrain
cd apps/website/frontend && npm run build && npm run lint
```

Manual: Mission Settings → **Map** → pan Everon; compare peninsula orientation vs Satellite; FpsCounter ≥55 fps.

---

## Return contract

- Commit SHA + tag **`T-090.1.1`**
- `.ai/artifacts/t090_1_1_source_spike.json`
- `.ai/artifacts/t090_1_1_verify_log.md` with M1–M9 PASS table
- **Ready for Cursor doc sync**
