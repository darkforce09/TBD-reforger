# T-152.4 — Fence / pier / bridge cartographic vectors

**Ticket:** T-152 · **Slice:** T-152.4  
**Status:** `ready` (blocked until **T-152.3** icons shipped — bridge deck glyph uses atlas)  
**Executor:** **grok-cursor** (Grok 4.5 in Cursor — **not** Claude Code)  
**Authority:** T-152 program hub (`.0` — written by parallel agent) · [`t090_phased_object_import.md`](t090_phased_object_import.md) · [`t090_render_lod_contract.md`](t090_render_lod_contract.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` (commit on `main` in worktree; tag **`T-152.4`**)  
**Depends on:** **T-152.3** (icon atlas keys for `building-bridge` badge) · prior T-152.0–.3 **all Gn PASS**  
**Baseline:** last shipped T-152.3 commit + verify log

## In one sentence

Export **P5_props** fence instances if missing, then render **thin OBB fence strips**, **oriented pier/dock planks**, and **bridge decks + railings** on the cartographic wgpu map with mathematical gates — no fat square piers, nonzero fence census.

---

## Problem

Everon export is capped at **`importPhaseMax: "P2_trees"`** in `packages/map-assets/everon/manifest.json` — **fence props (`kind=prop`, `propClass=fence`) are not in the committed catalog**. Piers/docks already export as `kind=water` with `class=pier|dock` and draw as building footprints (`render_class_for_prefab` in `crates/map-engine-core/src/world/classify.rs:18–24`), but they render as **filled OBB squares** via `building_prefab_lookup` + `obb_corners` (`crates/map-engine-core/src/world/obb.rs:37–71`) with pier tint in `fill_color` (`crates/map-engine-core/src/world/residency.rs:72–87`). That reads as **fat blocks**, not A3-style thin quays. **Bridges** (`buildingClass: "bridge"` in `packages/tbd-schema/schema/map-object-enums.schema.json:47`) get deck fill color but no dedicated cartographic treatment (deck strip + railing fence segments + optional bridge glyph). **Fences** are classified in `packages/tbd-schema/rules/prefab-classify.json` (`propClass: "fence"`) but never exported/rendered because P5 is not shipped.

Road strips already use `expand_polyline_strip` (`crates/map-engine-core/src/geometry/polyline_strip.rs:85–96`) — fence/railing geometry should reuse that lane, not invent a parallel TS path.

---

## Goal

1. **Data:** Run **`make map-export TERRAIN=everon PHASE=P5_props`** (Workbench step if staged raw missing) and bump manifest `importPhaseMax` / `importPhaseShipped` to include **`P5_props`** cumulatively (P1–P5).
2. **Fence lane:** For every exported instance with `propClass=fence`, emit a **thin oriented strip** along the OBB long axis (width **`FENCE_STRIP_WIDTH_M = 0.35`**, length = `2 × max(halfX, halfY)`), using `expand_polyline_strip` + `PolygonFill`/`Polyline` pipeline (same as roads).
3. **Pier/dock lane:** Replace square pier fills with **oriented thin strips** along the long OBB axis when **`aspect_ratio(halfX, halfY) ≥ PIER_ASPECT_MIN (= 4.0)`**; reject/skip rendering instances that fail the aspect predicate (no “fat square pier” on map).
4. **Bridge lane:** `buildingClass=bridge` instances draw **deck OBB fill** (existing `fill_color("bridge")`) **plus** `building-bridge` glyph at centroid (T-152.3 atlas) at `deckZoom ≥ BUILDING_BADGE_MIN_ZOOM` (`lodGates.ts:25`). **Railings:** sibling `propClass=fence` instances whose centroid is within **`BRIDGE_RAILING_RADIUS_M = 8`** of a bridge centroid compose as railing strips (or classify bridge prefab children in export spike — document path in verify log).
5. **LOD / toggles:** Honor `class_visible` for `prop` + `building`; new cartographic sub-toggle **`worldLayerPrefs.fences`** (default on) in Mission Settings layer panel.
6. **Gates:** Automated matrix G1–G10 below; verify log `.ai/artifacts/t152_4_verify_log.md`.

---

## Out of scope

- T-152 hub / slices `.0`–`.3` (parallel agent).
- Full **P10_full** export or new Workbench plugin classes (reuse `TBD_TerrainWorldExportPlugin.c` phase filter).
- Editor pick on fence segments (T-090.9 deferred).
- Retiring Deck `world-props` path (wgpu-only MC).
- Road class or runway work (**T-152.5**).
- Registry/doc sync (**Cursor** after Grok ships + verify PASS).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **P5_props required** if Everon `kind=prop` fence count = 0 in committed `prefabs.json.gz` | Manifest stuck at P2 |
| L2 | Fence/railing geometry = **`expand_polyline_strip`** in Rust (`polyline_strip.rs`) | Reuse road strip math; LANGUAGE GATE |
| L3 | **`FENCE_STRIP_WIDTH_M = 0.35`**; color `#8a8478` @ α0.85 (cartographic neutral) | A3 fence/wall readability |
| L4 | Pier strip predicate: **`max(hx,hy)/min(hx,hy) ≥ 4.0`**; strip width = **`min(hx,hy)×2`**; skip fill square when strip path taken | User gate: no fat square pier |
| L5 | Bridge deck keeps **`fill_color("bridge")`** (`residency.rs:77`); glyph key **`building-bridge`** from T-152.3 atlas | Icon slice dependency |
| L6 | Railing association: fence prop within **8 m** of bridge centroid **OR** export groups in spike — record **`G-railing-path A|B`** in verify log; **both PASS** if G7 met | Invent-or-spike for hard data gap |
| L7 | Draw order: buildings/bridge deck → **fence strips** → bridge glyph → trees (no z-fight with forest mass) | Cartographic clarity |
| L8 | LOD: fences visible `deckZoom ≥ PROP_MIN_ZOOM (3)`; pier strips `≥ BUILDING_FOOTPRINT_MIN_ZOOM (−2.5)`; bridge glyph `≥ BUILDING_BADGE_MIN_ZOOM (1)` | `lodGates.ts` |
| L9 | Vitest **+≥3** new tests (pier aspect, fence strip vertex count, bridge centroid glyph); wasm size logged, no hard cap | Regression |
| L10 | Commit **`T-152.4:`** · tag **`T-152.4`** · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| Everon `importPhaseMax` (today) | **P2_trees** | `packages/map-assets/everon/manifest.json:79` |
| P5 filter | `kind=prop` | `t090_phased_object_import.md:50` |
| `PIER_ASPECT_MIN` | **4.0** | This slice |
| `FENCE_STRIP_WIDTH_M` | **0.35** | This slice |
| `BRIDGE_RAILING_RADIUS_M` | **8.0** | This slice |
| Prop LOD min zoom | **3** | `lodGates.ts:29` |
| Building footprint min zoom | **−2.5** | `lodGates.ts:23` |

---

## Tasks

1. **Export gate:** If fence instance count = 0, run Workbench P5 export + `make map-export TERRAIN=everon PHASE=P5_props` + `make map-verify-phase`; update manifest phase fields.
2. **Rust — `FenceStripComposer`:** In `map-engine-core` (world or geometry module), map fence prop OBB → 2-point centerline + `expand_polyline_strip`.
3. **Rust — pier strip path:** Extend building compose in `residency.rs` (or dedicated `pier_strip.rs`) to emit thin strip instead of quad fill when aspect ≥ L4.
4. **Rust — bridge compose:** Deck polygon unchanged; queue bridge glyph instance; optional railing strip batch from nearby fence props.
5. **Render — wgpu lane:** Upload strip vertices; wire `WgpuTacticalMap` draw + `worldLayerPrefs.fences` toggle (thin TS).
6. **Tests + verify script:** `t152_4_fence_pier_bridge.test.ts` / Rust unit tests for predicates; write verify log; tag ship.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | After export, **`count(prefab.kind=prop ∧ propClass=fence) > 0`** on Everon | Export census |
| **G2** | **`count(instances of fence prefabs) > 0`** in chunked catalog | Export census |
| **G3** | **`make map-verify-phase TERRAIN=everon PHASE=P5_props` exit 0** | Phase script |
| **G4** | ∀ pier/dock instance rendered as strip: **`max(hx,hy)/min(hx,hy) ≥ 4.0`** | Geometry |
| **G5** | **`count(pier instances with aspect < 4.0 and drawn as square fill) = 0`** | No fat pier |
| **G6** | ∀ fence strip: triangle mesh width in world m = **`FENCE_STRIP_WIDTH_M ± 0.01`** at segment midpoint | Class R strip |
| **G7** | ∀ bridge instance: **`∃ ≥1 fence strip`** with centroid distance **≤ 8 m** **OR** verify log documents **G-railing-path B** (export-grouped) with rail segment count **≥ 2 × bridge_count** | Railing |
| **G8** | Bridge glyph: at pinned NW bridge camera, GPU readback **α > 0** at projected centroid | GPU-R |
| **G9** | `class_visible('prop')` at `deckZoom=3` → fence strips drawn; at `deckZoom=2.9` → hidden | LOD |
| **G10** | Prior T-152.0–.3 verify logs **PASS**; FE `npm test` + `npm run build` + `npm run lint`; `make wasm` exit 0 | Regression |

---

## Verify

```bash
# Preflight
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

# Export (if G1/G2 fail — requires Workbench staged raw)
make map-export TERRAIN=everon PHASE=P5_props
make map-verify-phase TERRAIN=everon PHASE=P5_props
make map-export-validate

# Rust / wasm
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm

# Fence census gate (G1/G2) — adjust path if script name differs after implement
node -e "
const gz=require('node:zlib');const fs=require('node:fs');
const p='packages/map-assets/everon/objects/prefabs.json.gz';
const j=JSON.parse(gz.gunzipSync(fs.readFileSync(p)));
const fences=(j.prefabs||[]).filter(x=>x.kind==='prop'&&x.class==='fence');
if(!fences.length) { console.error('G1 FAIL: no fence prefabs'); process.exit(1); }
console.log('G1 OK fence prefabs', fences.length);
"

# Frontend
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Cartographic **Map** view @ Everon harbor — pier reads as **thin quay**, not a fat rectangle.
- **M2:** Bridge crossing @ central Everon — **deck + rail lines** visible; glyph at zoom ≥ badge band.
- **M3:** Rural fence line @ zoom ≥ prop band — continuous thin stroke following terrain.
- **M4:** Toggle **Fences** off in layer prefs — strips hidden; buildings/roads unchanged.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.4 → shipped`; program hub W4 vector row; link verify log; `./scripts/ticket sync`; CLAUDE.md §Status bullet.

---

## Grok Code prompt — T-152.4 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.4** — fence / pier / bridge cartographic vectors.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain   # resolve or stash before ship
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm
  # Confirm T-152.3 verify log PASS before starting

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_4_fence_pier_bridge.md
  2. docs/specs/Mission_Creator_Architecture/t090_phased_object_import.md (P5_props)
  3. crates/map-engine-core/src/geometry/polyline_strip.rs
  4. crates/map-engine-core/src/world/{classify,obb,residency}.rs
  5. crates/map-engine-render/src/{engine.rs,lanes.rs}
  6. apps/website/frontend/src/features/tactical-map/worldmap/lodGates.ts
  7. packages/map-assets/everon/manifest.json
  8. packages/tbd-schema/rules/prefab-classify.json (fence rules)

═══ PROBLEM ═══
  Fences are not exported (manifest stuck P2). Piers draw as fat OBB squares. Bridges lack
  deck+railing cartographic treatment. Need P5 export + thin strips + pier aspect gate + bridge glyph.

═══ SHIPPED (do not reopen) ═══
  T-152.0–.3 per their verify logs (sequential gate G10).

═══ LANGUAGE GATE ═══
  Rust OWNS: fence/pier/bridge geometry, strip expansion, LOD predicates, GPU buffer compose, shaders.
  TypeScript ONLY: React layer toggle, thin wasm upload hooks, prefs wiring.
  STOP IF: about to implement strip math or pier aspect tests in .ts — move to map-engine-core.

═══ LOCKED ═══
  - P5_props export if fence count zero
  - FENCE_STRIP_WIDTH_M=0.35; PIER_ASPECT_MIN=4.0; BRIDGE_RAILING_RADIUS_M=8
  - expand_polyline_strip for fences/railings
  - fill_color bridge + building-bridge glyph (T-152.3)
  - G-railing-path A or B documented in verify log
  - Draw order + LOD per spec L7–L8

═══ DO ═══
  1. P5 export + manifest phase bump if needed (G1–G3)
  2. Rust pier strip + fence strip composers
  3. Bridge deck + railing association + glyph instance
  4. wgpu draw + worldLayerPrefs.fences toggle
  5. Tests for G4–G9; GPU-R G8
  6. .ai/artifacts/t152_4_verify_log.md; commit T-152.4: · tag T-152.4

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, TICKET_*.md, CLAUDE status markers
  - Implement T-152.5+ slices in this pass
  - Fat TS controllers for geometry policy
  - Skip P5 export while claiming fence ship

═══ VERIFY (all exit 0) ═══
  (copy bash block from spec §Verify)

═══ MANUAL ═══
  M1–M4 per spec

═══ RETURN ═══
  - Commit SHA + tag T-152.4
  - .ai/artifacts/t152_4_verify_log.md (G1–G10 + G-railing-path)
  - Automated verify PASS output
  - Manual M1–M4 notes
  - Ready for Cursor doc sync.
```
