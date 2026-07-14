# T-152.9 verify log — Road names (polyline-following labels)

**Slice:** T-152.9  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

Path **B** (curated `road-names.json`) after spike: `.topo` / `roads.json.gz` carry geometry only — no decodable name strings. Rust `world/road_labels.rs` places tangent-aligned labels (6 m offset, upright flip), declutters at `60·2^(-z)` m (cap 24), and packs rotated glyphs on `WorldRoadLabels` lane (below town labels, above roads stroke). wasm bridges + `WgpuRoadLabelController` + `worldLayerPrefs.roadNames` toggle (default on).

**Spike:** [`.ai/artifacts/t152_9_road_name_spike.json`](t152_9_road_name_spike.json) (`path: "B"`).

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | T-152.1 + T-152.8 verify logs **PASS** | **PASS** | [`.ai/artifacts/t152_1_verify_log.md`](t152_1_verify_log.md) · [`.ai/artifacts/t152_8_verify_log.md`](t152_8_verify_log.md) |
| **G2** | Spike JSON exists with **`path: "A"\|"B"`** | **PASS** | [`.ai/artifacts/t152_9_road_name_spike.json`](t152_9_road_name_spike.json) `path: "B"` |
| **G3** | **`MAJOR_EVERON_ROADS`** each **`∃ label`** fuzzy-match @ `deckZoom=0` | **PASS** | `verify-road-names.mjs` — 6/6 drawn |
| **G4** | **`∀ label: name.length ≥ 2`** | **PASS** | `road_name_schema_holds` + verify script |
| **G5** | Label center **`≤ 12 m`** perpendicular from polyline | **PASS** | `road_placement_geometry_holds` |
| **G6** | **`∀ pair: dist ≥ 60·2^(-z)`** @ z=0 | **PASS** | `road_declutter_invariant_holds` |
| **G7** | **`|on_screen| ≤ 24`** | **PASS** | drawn **13** @ z=0 |
| **G8** | Regression green | **PASS** | vitest **355/355**; FE build/lint OK; `make schema-validate` glyph `prop-unknown` FAIL **pre-existing** @ tip (unchanged by T-152.9) |

## Automated commands

```text
test -f .ai/artifacts/t152_9_road_name_spike.json                          → OK
cargo test -p map-engine-core road_labels --all-features                   → 7/7 PASS
cargo test -p map-engine-render draw_order --all-features                  → 7/7 PASS
make wasm                                                                  → map_engine_wasm_bg.wasm 4,327,185 B
cd apps/website/frontend && npm test                                       → 355/355 PASS
cd apps/website/frontend && npm run build && npm run lint                  → OK
node scripts/map-assets/verify-road-names.mjs --terrain everon --zoom 0    → OK
```

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Data path | **B** — `packages/map-assets/everon/road-names.json` |
| Curated named routes | **6** |
| Labels drawn @ z=0 | **13** |
| Glyph instances @ z=0 | **182** (3640 B packed) |
| `ROAD_NAME_DECLUTTER_BASE_M` | **60** |
| `ROAD_NAME_OFFSET_M` | **6** |
| `ROAD_NAME_MAX_ON_SCREEN` | **24** |
| Highway min zoom | **0** |
| Secondary min zoom | **1** (curated majors override via `minDeckZoom: 0`) |
| wasm merged size | **4,327,185** B |
| Cartographic tint | `#d8d4cc` @ α0.88 |

## MAJOR_EVERON_ROADS @ z=0 (G3)

Main Highway · North-South Highway · Coastal Road · Airfield Access · Gorey Road · Morton Road — all in drawn set.

## Manual (operator)

| ID | Status |
|----|--------|
| M1 | PENDING — central valley: at least one highway name readable along curve |
| M2 | PENDING — pan/rotate: labels stay glued to road |
| M3 | PENDING — toggle road names off |

Automated Gn all **PASS** — tag **T-152.9** allowed.
