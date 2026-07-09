# T-151.7 verify log — interaction rewire + parity suite (W7)

**Baseline:** tag **T-151.6** (`033ff715`) / worktree docs-sync `fb21b9dd`.  
**Slice:** W7 — ULP-0 camera on `WgpuTacticalMap`; same `useSelectTool` + page callbacks as Deck.

---

## What shipped

| Piece | Detail |
|-------|--------|
| Engine | `RenderEngine.unproject_xy` → `OrthoCamera` (screen CSS px → world m) |
| FE camera | `tools/mapCamera.ts` — `viewportFromViewState` (OrthoCameraJs snapshot), `viewportFromEngine`, `applyViewState`, `worldPickRadius` |
| Gesture SM | `useSelectTool` takes `getViewport()` — thresholds/modifiers unchanged (4 px, Ctrl toggle, marquee, pan MMB/RMB) |
| Deck path | `TacticalMap` injects `view.makeViewport` via `getViewport` |
| wgpu path | Removes raw LMB pan; hosts select tool + wheel `zoom_at` + viewState mirror `set_view` |
| Page | `MissionCreatorPage` passes same interaction props to wgpu as Deck |
| Picks | Unchanged `SlotIndex` 4 px + `ClusterIndex` 48 px |
| CUR z | rAF unproject + `sampleElevation` when DEM ready |
| Parity | `features/_wasm/interaction.parity.test.ts` (12 tests) |

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** |
| `cargo test -p map-engine-render` | **PASS** — 11 |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — **4,063,911 B** (T-151.6 was 4,063,618; **+293**) |
| `npm test` | **PASS** — **391** (+12 interaction parity; was 379) |
| `npm run build` + `lint` | **PASS** |
| entry isolation (`! grep map_engine_wasm_bg dist/assets/index-*.js`) | **PASS** |

### Parity suite (L7)

- Camera unproject Class R vs Deck / OrthoCameraJs (integer zooms, multi size/target)
- Pick radius 4 px world scale Class R (`r_world = 16 m @ zoom −2`)
- Selection scripts: click / Ctrl toggle / empty clear (T-053)
- Marquee `pickRect` Class S
- `encode_state` stable re-encode Class R; cross-doc move positions Class R
- Cluster 48 px world radius @ zoom −4
- CUR z == DEM sample when ready / 0 when not

---

## Manual (operator S1–S6)

| ID | Check | Status |
|----|-------|--------|
| **S1** | `?engine=wgpu` — click select, Ctrl toggle, empty clear | **operator** |
| **S2** | Marquee multi-select; Delete; undo restores rings + `slot_len` | **operator** |
| **S3** | Drag-move slots; no LMB-pan steal (MMB/RMB pan only) | **operator** |
| **S4** | Space flyTo; asset drop; dbl-click Attributes (≤1) | **operator** |
| **S5** | CUR X/Y/Z (DEM z when ready); cluster drill @ zoom ≤ −4 | **operator** |
| **S6** | Vitest interaction parity green (this log); browser A/B optional | **automated PASS** |

---

## Out of scope (locked)

- W8 cull / density ladder
- W9 Deck retirement
- Forest iso / slot GPU pack layout
- Docs/registry/CLAUDE (Cursor after)

---

## Ready for Cursor doc sync
