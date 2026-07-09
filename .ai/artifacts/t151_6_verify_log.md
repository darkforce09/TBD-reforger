# T-151.6 verify log — mission entities zero-copy (slots / selection / drag / clusters)

**Baseline:** tag **T-151.5.1** (`a98fb421`) / worktree docs-sync `00cc9b8a`.  
**Slice:** W6 — slot rings + selection tint + T-061 drag overlay + T-065 cluster discs on `WgpuTacticalMap` from MissionDoc SoA.

---

## Instance layout (unchanged 20 B)

| Field | Type | Bytes | Notes |
|-------|------|------:|-------|
| `pos` | `f32×2` | 8 | world meters → anchor-relative on upload |
| `size` | `f32` | 4 | **CSS pixels** for slots (× `px_to_m` in shader); world glyphs keep meters |
| `yaw` | `i16` snorm | 2 | 0 for rings/discs |
| `glyph` | `u16` | 2 | 0 = ring, 1 = disc (dedicated slot atlas) |
| `tint` | `u32` | 4 | RGBA8 `r\|g<<8\|b<<16\|a<<24` |
| **Total** | | **20** | `assert_eq!(size_of::<IconInstance>(), 20)` |

**IconUniforms (464 B):** UV[28] + `drag_delta: vec2` + `px_to_m: f32` + pad.  
World glyphs: `px_to_m=1`, `drag_delta=0`. Slots: `px_to_m=2^(-zoom)`. SlotDrag: live delta uniform.

Draw order: badges → **slots → slot-drag → clusters** → grid → marquee.

---

## What shipped

| Piece | Detail |
|-------|--------|
| Atlas | Procedural ring + disc (`slotAtlas.ts` → `upload_slot_atlas`) — **not** world-glyphs |
| Lanes | `LaneRole::{Slots, SlotDrag, Clusters}` |
| Engine APIs | `upload_slot_lane` / `patch_slot_lane` / `upload_slot_drag_lane` / `set_slot_drag_delta` / `upload_cluster_lane` / `set_slot_px_to_m` |
| Pack | `map-engine-core::slots_gpu` + FE mirror |
| FE bridge | `WgpuSlotsController` + `useWgpuSlots`; `missionDoc` prop from MissionCreatorPage |
| SoT | `MissionDoc.refresh()` → `slot_xy_ptr` / `slot_len` (never `slotsById` for GPU positions) |
| Selection | O(n) flag patch of size+tint; yellow 28 px / primary 20 px |
| Drag | T-061: hide base (α=0) + SlotDrag overlay + **16 B delta uniform** (`uniform_bytes_last_frame` = 80 while drag live) |
| Clusters | `slot_len > 500 && zoom ≤ −4`; discs from `getClusterMarkers` / Rust `ClusterIndex` |
| Stats | `slot_instances`, `slot_drag_instances`, `cluster_instances` |
| Dev | `window.__wgpuSlotStats` |

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** (incl. `slots_gpu` ×7) |
| `cargo test -p map-engine-render` | **PASS** — 11 |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — **4,063,618 B** (T-151.5.1 was 4,055,075; **+8,543**) |
| `npm test` | **PASS** — **379** (+5 slot pack/gate tests; was 374) |
| `npm run build` + `lint` | **PASS** |
| entry isolation (`! grep map_engine_wasm_bg dist/assets/index-*.js`) | **PASS** |

### Unit gates (L4)

- instance pack count == `xy.len()/2`
- selection size 28 + yellow tint bytes
- drag math `base + (dx,dy)`
- `cluster_mode` truth table vs `constants.ts` (500 / −4)
- `px_to_m = 2^(-zoom)` Class R

---

## Manual (operator / scripted)

W7 pick/gesture not wired — selection/drag tint via Zustand (console) or future dual-mount.

| ID | Check | Status |
|----|-------|--------|
| **S1** | `?engine=wgpu` mission with slots — rings @ zoom −2; count matches OBJ | **operator** |
| **S2** | Scripted `useMapStore.getState().setSelection({kind:'slot',ids:[…]})` → yellow; clear → primary | **operator** |
| **S3** | Scripted `setDragPreview` + `setDragPreviewDelta` → overlay follows; `uniform_bytes_last_frame` 80 | **operator** |
| **S4** | >500 slots, zoom ≤ −4 → cluster discs; zoom in → detail | **operator** |
| **S5** | Undo/redo → `slot_instances == slot_len` (`__wgpuSlotStats`) | **operator** |

Dev surface: `window.__wgpuSlotStats` after open `?engine=wgpu`.

---

## Out of scope (locked)

- useSelectTool / camera unproject (W7)
- Deck deletion (W9)
- Forest Path B / iso / T-149
- Docs/registry/CLAUDE (Cursor after)

---

## Ready for Cursor doc sync
