# T-152.8 — Town labels (locations + importance declutter)

**Ticket:** T-152 · **Slice:** T-152.8  
**Status:** `ready` (blocked until **T-152.1** text lane **and** **T-152.6** locations export PASS)  
**Executor:** **grok-cursor**  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.8`**  
**Depends on:** **T-152.6** (`locations.json`) · **T-152.1** (text lane) · **T-152.7** may ship parallel if text lane shared

## In one sentence

Render **town names** from `locations.json` on the cartographic map using the text lane and **A3-style importance-distance declutter** — all required Everon towns visible at island zoom.

---

## Problem

`locations.json` does not exist until **T-152.6**. Without labels, cartographic Map view fails Eden/A3 parity for settlement readability (`t144` §5: `_nearestMoreImportant ≥ 0.08·sizeLand·_scaleX`). Map export artifact references **Gorey, Highstone, RaccoonRock** as real towns. Text rendering requires **T-152.1** — not ad-hoc DOM overlays.

---

## Goal

1. **Loader:** Rust `world/locations.rs` parse `manifest.locations.path` JSON into `Vec<LocationLabel>`.
2. **Declutter:** Precompute `_nearestMoreImportant` analogue — for each location, distance to nearest **higher-importance** neighbor; draw iff **`nearest_more_important_m ≥ IMPORTANCE_SCALE · size_land_m · 2^(-deckZoom)`** with **`IMPORTANCE_SCALE = 0.08`** (A3 `uiMap.cpp:1684–1697`).
3. **`size_land_m`:** √(importance) × **`TOWN_BASE_SIZE_M = 400`** (tunable; log in verify).
4. **Style:** Cartographic sans via T-152.1; color `#e8e4dc` @ α0.92; halo `#1a1a1a` 1 px.
5. **Zoom band:** Labels visible **`deckZoom ≥ TOWN_LABEL_MIN_ZOOM (−3)`**; hide above **`TOWN_LABEL_MAX_ZOOM (2)`** if clutter (optional).
6. **Toggle:** `worldLayerPrefs.townLabels` (default on).
7. Verify log `.ai/artifacts/t152_8_verify_log.md`.

---

## Out of scope

- Height numbers (**T-152.7**).
- Road names (**T-152.9**).
- 3D scene labels (Eden "Toggle Location Labels 3D").
- Editing location names in editor.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **`REQUIRED_EVERON_TOWNS`** = same set as T-152.6 L1 | Consistency |
| L2 | At **`deckZoom = −2`** (default MC), **`REQUIRED_EVERON_TOWNS ⊆ drawn_set`** | User gate |
| L3 | Declutter invariant identical to G4 formula in matrix | Math |
| L4 | Name string = `locations.json` `name` field verbatim (trim whitespace) | Data SoT |
| L5 | Draw **above** height labels, **below** nothing critical | Z-order |
| L6 | Class **S** optional: compare drawn set vs golden at fixed zoom −2 | Parity |
| L7 | Tag **`T-152.8`** | Convention |

---

## Tasks

1. Parse locations in `WorldResidency` or dedicated store.
2. `importance_declutter.rs` pure function + tests.
3. Text batch per frame (only visible set changes on zoom).
4. TS toggle.
5. `verify-town-labels.mjs` for G2/G3.
6. Verify log.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | T-152.6 + T-152.1 verify logs **PASS** | Dependency |
| **G2** | At **`deckZoom=-2`**: **`REQUIRED_EVERON_TOWNS ⊆ drawn_names`** (fuzzy match) | Coverage |
| **G3** | **`∀ drawn pair: declutter predicate true`** at same zoom | Declutter |
| **G4** | **`∀ drawn: name.source = locations.json[id]`** | Provenance |
| **G5** | Toggle off → **`|drawn|=0`** | UI |
| **G6** | Pan/zoom −4…+1 — no text atlas leak / crash; FPS ≥ 55 @ default | Perf |
| **G7** | T-152.7 verify PASS (if shipped); regression green | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

test -f packages/map-assets/everon/locations.json || (echo 'G1 FAIL: run T-152.6' && exit 1)

cargo test -p map-engine-core importance_declutter --all-features
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint

node scripts/map-assets/verify-town-labels.mjs --terrain everon --zoom -2
```

---

## Manual acceptance

- **M1:** Default island view — read **Gorey**, **Morton**, **Levie** without zooming.
- **M2:** Zoom +4 — smaller hamlets hide before capital labels.
- **M3:** Toggle town labels — names disappear; height labels unaffected.

---

## Documentation sync (Cursor, after merge)

Registry; frontend map layer doc row; `./scripts/ticket sync`.

---

## Grok Code prompt — T-152.8 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.8** — town labels.

═══ PREFLIGHT ═══
  Confirm t152_6_verify_log.md + t152_1_verify_log.md PASS
  git lfs pull && make map-assets-link

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_8_town_labels.md
  2. docs/specs/Mission_Creator_Architecture/t152_6_locations_export.md
  3. packages/map-assets/everon/locations.json
  4. .ai/artifacts/t144_arma3_map_architecture_report.md §5
  5. T-152.1 text lane sources (spec from parallel agent)

═══ PROBLEM ═══
  Cartographic map lacks town names; consume locations.json with A3 declutter on text lane.

═══ LANGUAGE GATE ═══
  Rust: load locations, declutter math, text instances, GPU.
  TS: toggle only.

═══ LOCKED ═══
  - REQUIRED_EVERON_TOWNS at zoom -2
  - IMPORTANCE_SCALE=0.08; declutter invariant G3
  - worldLayerPrefs.townLabels

═══ DO ═══
  1. locations loader + importance declutter
  2. Text lane batching
  3. verify-town-labels.mjs; t152_8_verify_log.md · tag T-152.8

═══ DO NOT ═══
  - Re-export locations; DOM text; road/height labels

═══ VERIFY / RETURN ═══
  Per spec.
```
