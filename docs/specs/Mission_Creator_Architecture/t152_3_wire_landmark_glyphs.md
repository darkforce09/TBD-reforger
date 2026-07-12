# T-152.3 — Wire landmark building glyphs (residency + badges)

**Ticket:** T-152 · **Slice:** T-152.3  
**Status:** **queued**  
**Executor:** claude-code *(implementing agent: **Grok 4.5 in Cursor**)*  
**Authority:** [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · **Branch:** `ticket/T-152`  
**Depends on:** T-152.2 shipped · **Blocks:** T-152.4

---

## In one sentence

Expand **`badge_icon_key`** and **`rebuild_glyph_lookup_from_prefabs`** so **landmark building classes** draw center **glyphs** (not white-square-only footprints) at `deckZoom ≥ BUILDING_BADGE_MIN_ZOOM`, with **Class R** instance counts vs a TS oracle.

---

## Problem

| Issue | Evidence |
|-------|----------|
| Glyph lookup **skips buildings** | [`residency.rs`](../../../crates/map-engine-core/src/world/residency.rs) L276–316: only `tree`/`vegetation`/`prop`/`rockLarge` enter `glyph_by_u16`; `building` → `continue` |
| Badges only for **3 classes** | [`glyph_math.rs`](../../../crates/map-engine-core/src/world/glyph_math.rs) L127–135: `military`, `tower`, `bunker` |
| Lighthouse reads as **white fill** only | `fill_color("lighthouse")` → `[235,235,235,220]` L81; no center icon at badge zoom |
| `building-*` icons exist in atlas after T-152.2 but **never instanced** | Manifest keys unused by residency compose |

---

## Goal

1. **`building_icon_key(building_class) -> Option<&str>`** mapping every `LANDMARK_SET` building class to `building-{class}` or `building-badge-{class}` per [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md).
2. **`rebuild_glyph_lookup_from_prefabs`**: third group **building landmarks** (`group = 2`) when prefab `kind==building` and `iconKey` resolves.
3. **`rebuild_glyph_buffers`**: emit landmark glyphs into `badge_glyph_buf` (or dedicated `landmark_glyph_buf` if cleaner) at building instance centers when:
   - `class_visible("buildingBadge", z)` **and**
   - `badge_icon_key` **or** `building_icon_key` resolves.
4. **Footprint fill unchanged** — glyph overlays fill (not replacement).
5. **Class R** tests: pinned Everon chunk fixtures — glyph count per class matches TS oracle `buildingLayer` + `treePropLayer` badge policy.

---

## Out of scope

- Text labels (T-152.1 / .4+)
- `importanceZoom` coarse band (T-152.5)
- New SVG art (T-152.2)
- Pick/hover on landmarks (future)

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | `BUILDING_BADGE_MIN_ZOOM = 1.0` from [`lod_gates.rs`](../../../crates/map-engine-core/src/world/lod_gates.rs) — unchanged | N2 |
| L2 | Landmark glyph size = `badge_size_meters()` + `BADGE_SIZE_MIN_PX` clamp (same as military badge) | Visual parity |
| L3 | Tint = white `[255,255,255,255]` for non-tintable building icons; tree tint path unchanged | manifest `tintable:false` |
| L4 | Draw order: footprints → outlines → trees → props → **landmark/badges** → grid | T-151.5 L8 |
| L5 | `group=2` building glyphs count toward `badge_glyph_count()` stats (additive) | Regression |
| L6 | Class **R**: `|rust_count − ts_oracle| = 0` on fixture chunks @ z ∈ {1.0, 2.0, 3.0} | Parity |
| L7 | GPU-R optional: lighthouse @ (6400,6400) viewport — nonzero α at projected center | Advisory Mn |
| L8 | Tag **`T-152.3`** · verify `.ai/artifacts/t152_3_verify_log.md` | House |

---

## Tasks

| # | Path | Action |
|---|------|--------|
| 1 | `crates/map-engine-core/src/world/glyph_math.rs` | `building_icon_key`, extend `badge_icon_key` or unify |
| 2 | `crates/map-engine-core/src/world/residency.rs` | `rebuild_glyph_lookup_from_prefabs` building branch |
| 3 | `crates/map-engine-core/src/world/residency.rs` | `rebuild_glyph_buffers` landmark compose |
| 4 | `crates/map-engine-core/src/world/residency.rs` tests | Class R fixture tests |
| 5 | `apps/website/frontend/src/features/tactical-map/wgpu/` | Reload atlas if key order changes (should not) |
| 6 | `.ai/artifacts/t152_3_verify_log.md` | G1–G9 |

---

## Mathematical acceptance matrix

| ID | Predicate | Pass condition |
|----|-----------|----------------|
| **G1** | `building_icon_key` total | `∀ c ∈ BUILDING_CLASSES: building_icon_key(c) = Some("building-"+c)` or documented exception table empty |
| **G2** | Lookup population | After `load_prefabs_gz` + `set_glyph_key_map`, `|glyph_by_u16 ∩ building prefabs| ≥ N_min` where `N_min` = count of Everon building prefabs with `iconKey` in LANDMARK_SET (recorded in verify log, expect **≥ 15**) |
| **G3** | Zoom gate | @ `z=0.9`: `badge_glyph_count()==0` for landmark compose; @ `z=1.0`: `badge_glyph_count()>0` on pinned fixture |
| **G4** | Class R @ z=2.0 | `rust_badge_count == ts_oracle_badge_count` on fixture chunk `124_126` (or hub-pinned id) |
| **G5** | Class R @ z=2.0 landmarks | `rust_landmark_glyph_count == ts_oracle_landmark_count` for classes `{lighthouse, castle, bridge}` |
| **G6** | Lighthouse not fill-only | @ `z≥1`, lighthouse instances have `badge_glyph_buf` entry with `glyph_idx` for `building-lighthouse` |
| **G7** | Rust tests | `cargo test -p map-engine-core --all-features` exit 0 |
| **G8** | Wasm + FE | `make wasm` + `npm test && npm run build && npm run lint` exit 0 |
| **G9** | No regression | T-151 glyph LOD exhaustive scan still PASS |

**BUILDING_CLASSES** (normative):  
`residential, civic, agricultural, industrial, commercial, hangar, bunker, tower, military, bridge, castle, lighthouse, shed, container, tent, ruin, garage, generic`.

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual checklist

| ID | Check | Pass |
|----|-------|------|
| M1 | Everon lighthouse @ ~(4870,7760) — **icon visible** over white footprint @ zoom ≥ +1 | ☐ |
| M2 | Military site — badge icon, not tint-only square | ☐ |
| M3 | Zoom 0.5 — footprints only, no landmark glyphs | ☐ |

---

## Documentation sync (Cursor, after merge)

Registry `T-152.3 → shipped`; hub active **T-152.4**; author `t152_4_contour_elevation_labels.md`; `./scripts/ticket sync`.

---

## §Grok Code prompt — T-152.3 (copy-paste)

Authority: this spec + hub. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE (NOT main).

Implement **T-152.3** — wire landmark building glyphs in WorldResidency.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git branch --show-current
  git status --porcelain       # @ T-152.2 shipped
  git lfs pull && make map-assets-link && make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_3_wire_landmark_glyphs.md
  2. docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md
  3. docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md
  4. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md  (BUILDING_BADGE_MIN_ZOOM)
  5. crates/map-engine-core/src/world/{residency.rs,glyph_math.rs,lod_gates.rs}
  6. apps/website/frontend/src/features/tactical-map/worldmap/{buildingLayer,treePropLayer,lodGates}.ts
  7. packages/map-assets/glyphs/manifest.json

═══ PROBLEM ═══
  rebuild_glyph_lookup_from_prefabs skips buildings (else continue). badge_icon_key only maps
  military/tower/bunker. Lighthouse/castle/etc show fill_color only — white square for lighthouse.
  T-152.2 atlas has building-* icons but residency never instances them.

═══ SHIPPED (do not reopen) ═══
  T-152.2 — Reforger LANDMARK_SET art + atlas rebuild
  T-151.5 — IconInstanced pipeline + badge buffer compose

═══ LANGUAGE GATE (MANDATORY) ═══
  Rust OWNS: building_icon_key, glyph lookup group 2, badge/landmark compose, Class R tests.
  TypeScript ONLY: oracle tests / vitest parity scans — no new compose policy in TS.
  STOP IF: landmark visibility rules grow in .ts → move to lod_gates.rs / residency.rs.

═══ LOCKED ═══
  - building_icon_key for all BUILDING_CLASSES → building-{class}
  - Badge zoom gate: BUILDING_BADGE_MIN_ZOOM = 1.0
  - Footprint fill unchanged; glyph overlay at instance center
  - Class R: rust counts == TS oracle @ z∈{1,2,3} on pinned fixtures
  - stats badge_glyph_count includes landmark glyphs
  - Draw order unchanged (badges after props)

═══ DO ═══
  1. Add building_icon_key + extend rebuild_glyph_lookup_from_prefabs (group 2)
  2. Extend rebuild_glyph_buffers for all LANDMARK_SET classes @ badge zoom
  3. Class R unit tests (G4–G6) + vitest parity if applicable
  4. .ai/artifacts/t152_3_verify_log.md; commit T-152.3: · tag T-152.3

═══ DO NOT ═══
  - Edit packages/map-assets/glyphs/svg (T-152.2)
  - Edit docs/**, registry.json
  - Change BUILDING_BADGE_MIN_ZOOM without spec amendment
  - Remove footprint fills; ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace
  make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint

═══ MANUAL ═══
  M1: lighthouse icon over white footprint @ zoom ≥ +1
  M2: military badge visible
  M3: zoom 0.5 — glyphs off, footprints on

═══ RETURN ═══
  - Commit SHA + tag T-152.3
  - .ai/artifacts/t152_3_verify_log.md (G1–G9, N_min, fixture chunk ids)
  - **Ready for Cursor doc sync → T-152.4**
```
