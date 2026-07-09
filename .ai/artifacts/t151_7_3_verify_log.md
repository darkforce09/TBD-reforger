# T-151.7.3 verify log — Rust collapse: slot GPU bridge out of TypeScript

**Baseline:** HEAD after T-151.7.2 (`69ca1c08` + docs `5457dd4e`).  
**Worktree:** `tbd-reforger-wgpu-spike/` only.

---

## LOC proof

| File | Before | After |
|------|--------|-------|
| `wgpu/wgpuSlots.ts` | **521** | **56** (≤ 60) |
| `wgpu/slotAtlas.ts` | 154 (pack+atlas) | canvas atlas only |

```
$ wc -l apps/website/frontend/src/features/tactical-map/wgpu/wgpuSlots.ts
56 apps/website/frontend/src/features/tactical-map/wgpu/wgpuSlots.ts
```

No TS reimplementation of `pack_slot_instances` / `classifyDragTransition` / `cluster_mode` in `wgpuSlots.ts`.

---

## What shipped

| Layer | Change |
|-------|--------|
| `map-engine-core` `slots_gpu.rs` | Pure `classify_drag_transition`, `pack_selection_only`, `pack_drag_overlay`, `selected_mask`, `hide_slot_row_patch` + unit tests |
| `map-engine-render` `engine.rs` | `SlotGpuBridge` state on `RenderEngine`; high-level wasm API; low-level lane uploads **not** wasm-exported |
| `map-engine-wasm` | `bind_mission_doc(engine, doc)` borrow-only; pure FE smoke exports (`slot_cluster_mode`, `pack_slot_instances`, `px_to_m_at_zoom`, `classify_drag_transition`) |
| `wgpuSlots.ts` | Thin adapter ≤56 LOC |
| `slotAtlas.ts` | Canvas `buildSlotAtlas` only |
| FE tests | Wasm smokes + 7.2 pan-zoom; pack/phase SoT in cargo |

### Public wasm slot surface

- `ensure_slot_atlas`, `set_selection`, `set_drag`, `on_camera_changed`, `set_cluster_markers`, `cluster_mode`, `slot_stats_json`, `clear_slots`
- free: `bind_mission_doc`
- pure smokes: `slot_cluster_mode`, `pack_slot_instances`, `px_to_m_at_zoom`, `classify_drag_transition`

### Behaviors preserved (7.1 / 7.2)

- **Selection:** full rematerialize from SoA + mask (detail full-n; cluster selection-only). No OOB patch-by-index into short lanes.
- **Drag:** start/restart one overlay upload; per-frame = `set_slot_drag_delta` only.
- **Camera SoT:** unchanged in `WgpuTacticalMap` / `useSelectTool`; slots path uses `engine.zoom` for `px_to_m` + cluster gate.

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** (incl. slots_gpu 11) |
| `cargo test -p map-engine-render` | **PASS** — 11 |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — `map_engine_wasm_bg.wasm` **4,071,877 B** (was ~4,063,911) |
| `npm test` | **PASS** — **393** |
| `npm run build` + `lint` | **PASS** |
| entry isolation `! grep -l map_engine_wasm_bg dist/assets/index-*.js` | **PASS** |

---

## Manual (operator)

| ID | Check | Status |
|----|-------|--------|
| **S1** | Select / deselect / Ctrl-toggle — tint always matches | **operator** |
| **S2** | Drag ~1000 — FPS OK; delta-only per frame | **operator** |
| **S3** | RMB + wheel — zoom-at-cursor; scroll wheel | **operator** |
| **S4** | `wgpuSlots.ts` ≤ 60; no TS pack policy | **PASS** (automated) |

Hard-refresh after pull (`?engine=wgpu`).

---

## Out of scope (locked)

- Supercluster in Rust (FE still feeds `set_cluster_markers`)
- Gesture SM (`useSelectTool`) in Rust
- W8 / W9
- Docs/registry/CLAUDE (Cursor after)

---

## Ready for Cursor doc sync
