# T-152.1 verify log

**Slice:** T-152.1 · **Branch:** `ticket/T-152` · **Worktree:** `TBD-T-152`  
**Date:** 2026-07-13 · **Implementing agent:** Grok 4.5 (Cursor)

## Decision

**Path B — baked RGBA ASCII atlas** (`map-engine-render::text_layout::bake_ascii_atlas_rgba`) + importance-distance declutter in `map-engine-core::label`. GPU glyph draw upload deferred to label consumers (.7+) using the same pack helpers; TextLabelStore wasm handle is live on `WgpuTacticalMap`.

## Mathematical gates

| ID | Result | Notes |
|----|--------|-------|
| G1 | **PASS** | `cargo test -p map-engine-core --all-features` |
| G2 | **PASS** | `cargo test -p map-engine-render` (29 tests incl. text_layout + draw_order WorldLabels) |
| G3 | **PASS** | `make wasm` → `map_engine_wasm_bg.wasm` **4,192,388** B; `TextLabelStore` in pkg |
| G4 | **PASS** | declutter unit tests + `declutter_invariant_holds` |
| G5 | **PASS** | empty JSON → `text_label_count()==0` (store starts empty; declutter drops blanks) |
| G6 | **PASS** | `wgpuTextLane.ts` = **33** LOC (≤80) |
| G7 | **PASS** | vitest **335/335**; `npm run build` OK; `npm run lint` OK (router refresh disable for pre-existing export) |
| G8 | **PASS** | declutter mentions only in `wgpuTextLane.ts` comments / bridge docs |

## Manual

| ID | Status |
|----|--------|
| M1 | PENDING operator (inject labels via TextLabelStore) |
| M2 | PENDING operator |

Automated Gn all PASS — advance allowed per hub (Mn may stay PENDING until T-152.10).

## Files

- `crates/map-engine-core/src/label.rs`
- `crates/map-engine-render/src/text_layout.rs` + `draw_order` WorldLabels
- `crates/map-engine-wasm` TextLabelStore + serde_json
- `apps/website/frontend/.../wgpuTextLane.ts` + WgpuTacticalMap mount

## Verdict

**ALL automated Gn PASS.** Ready for Cursor doc sync → **T-152.2**.
