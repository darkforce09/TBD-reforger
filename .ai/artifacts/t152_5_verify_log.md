# T-152.5 verify log — Airfield symbology (runway / apron / structures)

**Slice:** T-152.5  
**Branch:** `ticket/T-152`  
**Worktree:** `/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

NW Everon airfield cartographic read shipped: **runway polish** (20 m, `#b8bdb8` / `#6a706e` casing), **DEM-flat apron** polygon (`world-airfield-apron`, role **8**), **hangar/tower** landmark glyphs inside runway bbox (T-152.3 keys), **`worldLayerPrefs.airfield`** toggle. Taxiway spike **path B** — no taxiway linework in export; runway + apron + structures only (A3-like).

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | `count(roadSegments where roadClass=runway) ≥ 5` | **PASS** | **5** runway segments (`roads.json.gz`; `WorldStore.runway_segment_count`) |
| **G2** | Apron area `> 0` and `≥ 15_000` m² | **PASS** | Everon DEM @ `dem_apron_grid_factor=16`: **36,020** m² qualifying cells; mesh `polygon_count > 0` (`t152_5_airfield_symbology.test.ts`) |
| **G3** | ∃ hangar glyph in bbox @ `deckZoom=2` | **PASS** | `badge_glyph_count > 0` with airfield toggle on + bbox from runways (`t152_5_airfield_symbology.test.ts`) |
| **G4** | ∃ tower glyph in bbox @ `deckZoom=2` | **PASS** | Same residency smoke @ z=2; tower key `building-badge-tower` (T-152.3) |
| **G5** | Runway screen width @ `deckZoom=0` = 20 m ± 0.05 | **PASS** | Rust `polyline_strip::runway_polish_width_at_zoom_zero` Class R |
| **G6** | `airfield=false` → apron polish + icons hidden; roads unchanged | **PASS** | `set_airfield_toggle(false)` drops badge count; `road_segment_count` unchanged; `worldLayerPrefs` persist test |
| **G7** | `G-taxiway-path A` or `B` with spike JSON | **PASS** | **Path B** — [`.ai/artifacts/t152_5_taxiway_spike.json`](t152_5_taxiway_spike.json) |
| **G8** | T-152.4 PASS + FE build/lint/test + `make wasm` | **PASS** | T-152.4 verify ALL PASS; vitest **355/355**; build/lint clean; `make wasm` exit 0 |

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Runway segments | **5** |
| Airfield bbox (m) | **[4677, 6256, 5370, 12087]** (+30 m runway union margin) |
| Apron qualifying area | **36,020** m² |
| `APRON_DEM_DOWNSAMPLE_FACTOR` | **16** (32 m cells; σ gate on flattened pad) |
| `RUNWAY_POLISH_WIDTH_M` | **20** |
| Apron fill | `#9aa3a2` @ α0.55 |
| Draw order | apron (6) → roads casing (7) → roads (8) → buildings → icons |

## Taxiway spike (G7)

**Path B** — `.topo` type-0 is runway-only (5 records); no `roadClass=taxiway` in `roads.json.gz`; no RoadEntity export path. Operator note: taxiway-less NW read acceptable per spec M4.

## Automated commands

```text
git lfs pull && make map-assets-link  → 0
G1 node census  → G1 OK runways 5
cargo fmt --check  → 0
cargo clippy -p map-engine-core -p map-engine-render -p map-engine-wasm --features world -- -D warnings  → 0
cargo test -p map-engine-core --features world  → 152/152
cargo test -p map-engine-render  → 31/31
make wasm  → 0
cd apps/website/frontend && npm test  → 355/355
npm run build && npm run lint  → 0
```

**Wasm size:** `map_engine_wasm_bg.wasm` = **4,214,324 B**

## Manual (operator)

| ID | Check | Pass |
|----|-------|------|
| M1 | Island zoom Map — NW airfield pale pad + runway cross | ☐ |
| M2 | Zoom airfield — hangar row + control tower icons | ☐ |
| M3 | Toggle **Airfield** off — pad/runway polish/icons disappear | ☐ |
| M4 | Path B — taxiway-less read acceptable A3-like | ☐ |

## Prior slices

| Slice | Result |
|-------|--------|
| T-152.0–.4 | PASS per respective verify logs |

## Ready for

**T-152.6** locations export · **T-152.7** height markers
