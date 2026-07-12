# T-152.3 verify log — Wire landmark building glyphs

**Slice:** T-152.3  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

Landmark building classes now compose center glyphs (or military/tower/bunker badge overlays) into `badge_glyph_buf` at `deckZoom ≥ BUILDING_BADGE_MIN_ZOOM` (1.0). Footprint fills unchanged.

**Changes:**
- `glyph_math.rs`: `BUILDING_CLASSES`, `building_icon_key`, `landmark_glyph_icon_key`
- `residency.rs`: `rebuild_glyph_lookup_from_prefabs` group `2` for buildings; `rebuild_glyph_buffers` uses `landmark_glyph_icon_key`
- Class R tests on Everon fixture chunk **`2_12`** (hub-pinned; spec `124_126` absent on Everon grid)
- TS oracle: `buildingGlyphs.ts` + vitest; wasm smoke parity test

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | `building_icon_key` total | **PASS** | `glyph_math::building_icon_key_covers_all_building_classes` + `buildingGlyphs.test.ts` |
| **G2** | Lookup population ≥ N_min | **PASS** | `glyph_by_u16` group-2 = **213** (N_min = 15; Everon building prefabs with resolvable `iconKey`) |
| **G3** | Zoom gate | **PASS** | @ z=0.9 `badge_glyph_count()==0`; @ z=1.0 fixture `badge_glyph_count()>0` (`g3_zoom_gate_*`, wasm smoke) |
| **G4** | Class R badge @ z∈{1,2,3} | **PASS** | `g4_class_r_badge_counts_match_oracle` chunk `2_12`: rust == TS oracle |
| **G5** | Landmark subset @ z=2 | **PASS** | `{lighthouse,castle,bridge}` oracle **11** on chunk `2_12`; composed landmark glyph indices ≥ oracle |
| **G6** | Lighthouse glyph | **PASS** | `building-lighthouse` `glyph_idx` present in `badge_glyph_buf` @ z=2 |
| **G7** | Rust tests | **PASS** | `cargo test -p map-engine-core --all-features` → **163/163** |
| **G8** | Wasm + FE | **PASS** | `make wasm`; vitest **339/339**; `npm run build`; `npm run lint` |
| **G9** | LOD regression | **PASS** | `lod_gates::exhaustive_zoom_scan_glyph_classes_stable` + `lodGates.test.ts` bands unchanged |

## Pinned fixtures

| Item | Value |
|------|-------|
| Fixture chunk | `2_12` |
| Buildings in chunk | 69 |
| Landmark instances (lighthouse/castle/bridge) | 11 |
| Draw-set badge count @ z=2 (strict viewport) | 70 (chunk `2_12` + neighbor `3_12` in draw_ids) |
| `N_min` (building prefabs w/ landmark `iconKey`) | 213 |
| `glyph_by_u16` group-2 entries | 213 |

## Automated commands

```text
cargo fmt (map-engine-core)
cargo clippy -p map-engine-core --all-features --all-targets -- -D warnings  → 0
cargo test -p map-engine-core --all-features  → 163/163
cargo test -p map-engine-render  → 29/29
make wasm  → 0
cd apps/website/frontend && npm test  → 339/339
npm run build && npm run lint  → 0
```

**Wasm size:** `map_engine_wasm_bg.wasm` = **4,193,922 B**

## Manual (operator)

| ID | Check | Pass |
|----|-------|------|
| M1 | Lighthouse icon over white footprint @ ~(4870,7760) zoom ≥ +1 | ☐ |
| M2 | Military site — badge icon visible | ☐ |
| M3 | Zoom 0.5 — footprints only, no landmark glyphs | ☐ |

## Ready for

Cursor doc sync → **T-152.4**
