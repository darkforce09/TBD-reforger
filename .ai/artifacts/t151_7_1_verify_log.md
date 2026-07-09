# T-151.7.1 verify log — interaction hotfix (selection tint / drag FPS / zoom-at-cursor)

**Baseline:** tag **T-151.7** (`ab6bcb11`) / worktree docs-sync `d1ed26e2`.  
**Slice:** Three operator regressions after W7 — no gesture redesign.

---

## Root causes → fixes

| Bug | Cause | Fix |
|-----|--------|-----|
| **B1** tint | Cluster short-lane (k rows) patched by full-doc index → silent OOB | Cluster: re-upload selection-only; flag `slotsLaneIsSelectionOnly`; detail keeps patch |
| **B2** drag FPS | Every delta → full `syncDrag` + `create_buffer_init` | `classifyDragTransition`: start/restart upload once; delta = `set_slot_drag_delta` only |
| **B3** zoom | Wheel used canvas rect; pan used container | Wheel + listener on **container** rect (same CSS origin) |

---

## What shipped

| File | Change |
|------|--------|
| `wgpu/wgpuSlots.ts` | B1 + B2; `classifyDragTransition` exported |
| `WgpuTacticalMap.tsx` | B3 wheel container origin; HUD **T-151.7.1** |
| `slotGpu.parity.test.ts` | +1 phase classification test |

**Unchanged:** gesture SM thresholds, forest/slot atlas, engine buffer API (FE no longer re-uploads drag each frame).

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
| `make wasm` | **not required** (no engine/wasm surface change; baseline **4,063,911 B**) |
| `npm test` | **PASS** — **392** (+1; was 391) |
| `npm run build` + `lint` | **PASS** |
| entry isolation | **PASS** |

### Unit

- `classifyDragTransition`: start / delta / end / restart / idle truth table

---

## Manual (operator S1–S3)

| ID | Check | Status |
|----|-------|--------|
| **S1** | Rapid select / Ctrl-toggle / deselect — tint always matches (detail + cluster) | **operator** |
| **S2** | Drag ~1000 — FPS drop ≪ 40; `uniform_bytes_last_frame` ~80 during drag | **operator** |
| **S3** | RMB-hold + wheel — world under cursor stays put | **operator** |

Dev: `window.__wgpuSlotStats` now includes `slots_lane_selection_only`, `drag_active`.

---

## Out of scope (locked)

- W8 / W9
- Gesture redesign
- Docs/registry/CLAUDE (Cursor after)

---

## Ready for Cursor doc sync
