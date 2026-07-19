# T-175 — MC interaction + LOD + pan/zoom perf (post T-174 eye-pass)

**Status:** SHIPPED @ tag **T-175** / `b90deac8` · **Branch:** `main`  
**Depends on:** T-174 (shipped)  
**Verify:** [`.ai/artifacts/t175_verify_log.md`](../../.ai/artifacts/t175_verify_log.md) · inventory [`.ai/artifacts/t175_inventory.md`](../../.ai/artifacts/t175_inventory.md)  
**Scope shipped:** `apps/website/frontend/**`, `crates/map-engine-*`, gates tooling as needed. **Not** `apps/mod/`.

**No silent deferrals.** Soft “later / optional / fold forward / separate ticket” is forbidden unless the operator explicitly says `defer X` / `skip X`.

## Shipped outcome

| ID | Result |
|----|--------|
| A1 | Sticky trees fixed — engine empty upload clears IconInstanced every role; host clears on residency `*_lane_off` (LOD-off), not `pin_settled`. |
| A2 | Forest settle — `fetch_missing` concurrent batch + progressive push (was serial despite `FETCH_CONCURRENCY`). |
| A3 | Contours — `CONTOUR_RGBA` `[90,70,40,180]` → `[188,150,100,235]` (1px width fixed by wgpu). |
| A4–A5 | Zoom/pan — floor-aware glyph memo, strip own key, heatmap above memo, static importance breakpoints. |
| B1 | Cold slots — `rebind_engine_from_doc` + 2-cell restore/engine handshake. |
| B2 | Place ghost — engine `SlotPlacePreview` lane + FE wire. |
| B3 | Drag preview — `damage.mark()` in `set_slot_drag_delta` (root cause). |
| B4 | Selection — O(delta) tint patch vs O(n) rematerialize. |
| B5 | Boot — `BootPhase` overlay until hydrate + world settle. |
| C | Hunt H1–H6 (dead density-pack, zoom_changed, concurrent fetch, dup const, damage-mark audit, building guard). |

**Gates:** `wasm-ci` · `ci-local-leptos` · `leptos-gates` 18+24 · `ci-local` EXIT 0 · perf-strict encode floor.

**Manual visual (operator):** SwiftShader proves mechanism; on-GPU `rf` / `__editorBench` + eye-pass on `make leptos` still operator G-A — zoom-out unpacks trees · forest fills fast · contours readable · cold-load slots · place ghost · live drag · selection snappy · boot loading bar.

## Why (pre-ship)

T-174 sat/heatmap/guides looked good. Remaining everyday MC pain: zoom/pan stutter; sticky tree glyphs on zoom-out; forest settle delay; dark contours; selection lag; wrong first-load slot positions; no palette place ghost; slot drag ~1 px then jump; no boot loading UX. Plus mandatory experience hunt.

## Agent split (HARD)

| Who | Owns |
|-----|------|
| **Claude Code** | Inventory + all code fixes + gates + verify log + tag **T-175** |
| **Cursor** | Spec, handoff, registry; post-ship doc sync |

## Operator matrix (met)

### A — LOD / world layers

| ID | Bug | Shipped fix |
|----|-----|-------------|
| A1 | Tree glyphs sticky on zoom-out | Empty GPU clear all roles + host clear on `*_lane_off` |
| A2 | Forest settle too slow | Concurrent fetch + progressive push |
| A3 | Contours too dark | Lighter `CONTOUR_RGBA` |
| A4–A5 | Pan/zoom stutter | Glyph/strip memo + heatmap ordering |

### B — Interaction / document

| ID | Bug | Shipped fix |
|----|-----|-------------|
| B1 | Cold-load wrong slot positions | Rebind handshake |
| B2 | No palette place ghost | `SlotPlacePreview` |
| B3 | Drag preview ~1 px | `damage.mark` on drag delta |
| B4 | Selection laggy | O(delta) tint |
| B5 | No loading bar | `BootPhase` overlay |

### C — Experience hunt (met)

H1–H6 documented in inventory + verify Found-by-hunt (non-empty).

## Locked (historical)

1. Fix all A/B + inventory rows — no silent defer.  
2. Sticky-empty must not block GPU clears.  
3. Language gate: engine/LOD/drag GPU in Rust.  
4. No density-heatmap green glow (T-174).  
5. `apps/mod/**` OFF LIMITS.  
6. Measure on `make leptos` (release).  
