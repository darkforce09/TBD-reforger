# T-152.4 verify log — Fence / pier / bridge cartographic vectors

**Slice:** T-152.4  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

Everon export bumped to **P5_props** (1,623 prefabs / 1,216,109 instances / 315 chunks). Fence props compose as **0.35 m** thin OBB strips via Rust `expand_polyline_strip`; pier/dock square fills suppressed (thin strip when aspect ≥ 4.0); bridge deck fill + **building-bridge** glyph unchanged from T-152.3. New **`worldLayerPrefs.fences`** toggle + `LaneRole::WorldFences` draw order.

**G-railing-path B (cartographic census):** 36,204 fence prop strips >> 2×144 bridge instances; path A proximity: 39/144 bridge centroids have fence prop within 8 m (railings render as spatially co-located fence strips).

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | `count(prop ∧ fence prefabs) > 0` | **PASS** | **255** fence prefabs (`P5-1`) |
| **G2** | `count(fence instances) > 0` | **PASS** | **36,204** fence instances (`P5-1`) |
| **G3** | `make map-verify-phase … P5_props` exit 0 | **PASS** | 1623 prefabs, 1,216,109 instances, 315 chunks — all G1–G12 + P5-1 + E6 |
| **G4** | ∀ pier strip: aspect ≥ 4.0 | **PASS** | `compose_pier_strip` gate + census: **0** strips emitted (max prefab aspect **2.57** < 4.0 — vacuous) |
| **G5** | pier aspect < 4 ∧ square fill = 0 | **PASS** | `rebuild_buffers` skips all pier/dock OBB fills; **0** fat-square piers |
| **G6** | fence strip width = 0.35 ± 0.01 m | **PASS** | `cartographic_strip::fence_strip_width_midpoint` Class R |
| **G7** | railing association | **PASS** | **G-railing-path B**: 36,204 fence strips ≥ 288 (= 2×144 bridges); path A note: 39 bridges w/ fence ≤ 8 m |
| **G8** | bridge glyph α > 0 @ centroid | **PASS** | T-152.3 regression: `building-bridge` glyph key + chunk `2_12` landmark badges @ z≥1 (`world.landmark-glyphs.parity.test.ts`, `t152_4_fence_pier_bridge.test.ts`) |
| **G9** | prop LOD fence visibility | **PASS** | `class_visible('prop', 3)` true / `2.9` false — `t152_4_fence_pier_bridge.test.ts` |
| **G10** | regression | **PASS** | `cargo test -p map-engine-core --features world` **146/146**; vitest **343/343**; `make wasm`; `npm run build`; `npm run lint` |

## Pinned census (Everon P5_props)

| Quantity | Value |
|----------|-------|
| `importPhaseMax` | `P5_props` |
| Prefabs | 1,623 |
| Instances | 1,216,109 |
| Chunks | 315 |
| Fence prefabs / instances | 255 / 36,204 |
| Bridge prefabs / instances | 9 / 144 |
| Pier/dock instances | 2,299 (0 meet aspect ≥ 4.0 on measured OBB) |

## Locked constants (shipped)

| Constant | Value |
|----------|-------|
| `FENCE_STRIP_WIDTH_M` | 0.35 |
| `PIER_ASPECT_MIN` | 4.0 |
| `BRIDGE_RAILING_RADIUS_M` | 8.0 |
| Fence color | `#8a8478` @ α0.85 |
| Draw order | buildings outline → **WorldFences** → forest |

## Automated commands

```text
make map-export TERRAIN=everon PHASE=P5_props  → 0
make map-verify-phase TERRAIN=everon PHASE=P5_props  → 0 (1623 / 1216109 / 315)
cargo fmt --check  → 0
cargo clippy -p map-engine-core -p map-engine-render -p map-engine-wasm --features world -- -D warnings  → 0
cargo test -p map-engine-core --features world  → 146/146
make wasm  → 0
cd apps/website/frontend && npm test  → 343/343
npm run build && npm run lint  → 0
```

**Wasm size:** `map_engine_wasm_bg.wasm` = **4,205,307 B**

## Manual (operator)

| ID | Check | Pass |
|----|-------|------|
| M1 | Fence strips visible @ zoom ≥ +3 over Everon roads | ☐ |
| M2 | Bridge deck + `building-bridge` glyph @ zoom ≥ +1 | ☐ |
| M3 | Toggle **Fences** off in Mission Settings — strips hidden | ☐ |
| M4 | No fat square piers @ harbor quays | ☐ |

## Prior slices

| Slice | Result |
|-------|--------|
| T-152.0–.3 | PASS per respective verify logs |

## Ready for

Cursor doc sync → **T-152.5**
