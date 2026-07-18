# T-175 — verify log (MC interaction + LOD + pan/zoom perf)

Baseline **T-174 @ `bbb99526`**. All work on `main`, `apps/website/**` + `crates/map-engine-*`.
Inventory: [`t175_inventory.md`](t175_inventory.md). Language gate held — LOD / residency / glyph
pack-clear / drag GPU / camera math in Rust (`crates/map-engine-*`); Leptos = pointer / chrome /
thin engine calls.

## Operator word (2026-07-18) → row

> zoom/pan stutter · tree glyphs sticky on zoom-out · forest mass delay too long · contours too dark ·
> selection laggy · first-load slots wrong · palette drag no ghost · slot move ~1 px then jump ·
> loading bar/screen · "hunt for other optimization/LOD/interaction holes."

All addressed below. **B5 scope confirmed IN by the operator this session** (the pasted prompt listed
B1–B4; the authoritative spec + handoff list B5 — quoted authorization for including it).

## Gates (all green)

| Gate | Result |
|------|--------|
| `make wasm-ci` | fmt-check + clippy **`-D warnings`** (core/wasm + **render wasm32**) + core **247** + render **45** tests — PASS |
| `make ci-local-leptos` | fmt + clippy wasm32 + **73** frontend tests + **trunk build --release** (23.9 s) — PASS |
| `make leptos-gates` | **18/18** editor smokes `pass:true` + **V-suite 24/24** pages `diffs=0` — PASS |
| `make ci-local` | **EXIT 0** (editorconfig · no-python · no-node · rust-ci backend fmt/clippy/build/**test-it** · coding-standards · leptos · schema + citations) |
| `gate smoke perf-strict` | `S_bench_encode_60_floor` + `S_dup_fetches_zero` + `S_idle_fetches_zero` + `settled` — all **true** |

Editor smokes proving no interaction/LOD regression: **fullmap** (`tree_glyphs===0` census at island),
**marquee-drag** (move digest + edit-persist), **select**, **pan**, **hydrate**, **cur**,
**attributes**, **doc**, **persist**, **save-export**, **undo**, **arsenal** — all pass.

## A — LOD / world layers

| ID | Done when | Result | Mechanism / evidence |
|----|-----------|--------|----------------------|
| **A1** | zoom-out past glyph band → trees **gone** from GPU (not the sticky last-zoom-in set) | **PASS** (code + fullmap gate + unit) | Two leaks fixed: engine `upload_icon_lane` empty branch now clears the IconInstanced batch for **every** role regardless of `visible` (was WebGPU-tree-only; props/badges + WebGL trees leaked); host `world_host.rs` guard clears when residency `*_lane_off()` says LOD-off, independent of `pin_settled` (island pin set kept it false → sticky). Residency exposes `tree_lane_off`/`prop_lane_off`/`badge_lane_off` (`!want ∥ heatmap_trees`). `smoke_fullmap` still asserts `tree_glyphs===0` at island; zoom-in probe `tree_glyphs>0`. |
| **A2** | forest mass appears after zoom-out without a long dead pause | **PASS** (code + zero-thrash gate) | Root cause = `forest_mass.rs::fetch_missing` awaited every `.bin` **serially** despite `FETCH_CONCURRENCY`. Now fetches each batch **concurrently** (`join_all`) + pushes a partial composite after each batch (progressive fill via the existing `present`-count memo). ~12× fewer serial RTTs on an island zoom-out. `S_idle_fetches_zero` = true (no thrash). |
| **A3** | contours clearly readable on Everon sat at default + mid zoom | **PASS** (code) — operator visual G-A | `CONTOUR_RGBA [90,70,40,180]` (luma ~72) → **`[188,150,100,235]`** (luma ~155, α235) — readable over dark sat + tan Map basemap. wgpu draws contours as a native 1 px LineList (width not a lever). `contour_segments>0` gate unaffected. |
| **A4/A5** | pan/zoom stutter materially reduced; no full recompose every wheel tick | **PASS** (code + pan gate + perf-strict) | **(1)** Split compose memo (`residency.rs`): `glyph_base_sig` gates the ≤150 k tree/prop/badge pack with a **floor-aware zoom term** — a sentinel above `glyph_size_floor_zoom` (where min-px sizes are constant) so a fine sub-chunk zoom in-band is a **memo hit** (no repack), exact zoom below; strip lane keeps its own key. `heatmap_trees` moved above the memo (fresh). Importance-override badges use a static `importance_breakpoints` activation index (no raw-zoom bust). **(2)** H1: the orphaned per-recompose `pack_density_grid_r32` (625 `format!` allocs/repack, no reader) removed from the hot path. **(3)** A1 removes the sticky-tree lane churn that dominated island pan/zoom. Unit `a5_glyph_memo_hits_in_band_busts_on_cross` + `a5_floor_zoom_term_*`. |

## B — interaction / document

| ID | Done when | Result | Mechanism / evidence |
|----|-----------|--------|----------------------|
| **B1** | cold load with slots → icons at correct world positions on first paint | **PASS** (code + hydrate gate) | The IDB-restore swap called only `refresh_hud()` (no engine rebind) and raced the engine first-bind → seed positions stuck. New `mission_history::rebind_engine_from_doc()` (rebind without dirty/persist/doc_ver) + a two-`Cell` handshake (`restore_settled`/`engine_mounted`) → the second-to-settle party binds once from the settled doc. `smoke_hydrate` PASS. |
| **B2** | palette asset over map shows a live ghost; release commits there | **PASS** (code) — operator visual G-A | New engine `LaneRole::SlotPlacePreview` (above Slots, below drag; pin `place_preview_sits_above_slots_below_drag`) + `set_place_preview`/`clear_place_preview` (translucent slot ring, shared `px_to_m`, marks damage). FE draws it in the pointermove `has_pending()` branch (same frozen-camera unproject as CUR); cleared on drop / cancel / pointer-leave. |
| **B3** | dragging a slot shows continuous live preview; release commits (one undo) | **PASS** (code + marquee-drag gate) | NOT units (verified world-consistent end-to-end). Root cause = the damage-driven engine's `set_slot_drag_delta` wrote the drag uniform without `damage.mark()` → no repaint until commit ("~1 px then teleport"). Added `damage.mark()` there **and** in `set_slot_px_to_m` (zoom-during-drag, H5). `smoke_marquee_drag` move digest + edit-persist PASS. |
| **B4** | selection / marquee tint feels immediate | **PASS** (code + select/marquee gates) | `set_selection` re-packed **all n** slot instances every click/outliner select. Now O(delta): diff old/new selected set, patch only the 12 B tint/size block (`row·20+8`) for flipped rows via `patch_slot_lane` + `damage.mark()`; full rematerialize only for cluster/selection-only + drag. Patched bytes are byte-identical to a full pack (`selected_row_patch`/`unselected_row_patch`). `smoke_select` + `smoke_marquee_drag` PASS. |
| **B5** | cold open shows a loading screen/bar until hydrate + initial map readiness | **PASS** (code) — operator visual G-A | New reactive `BootPhase` (`Hydrating`→`LoadingMap`→`Ready`) threaded from the two boot tasks + the world bootstrap (`world_ready`); full-bleed overlay (phase label + the surviving T-060 `mc-load-bar`) until doc hydrate **and** world/residency settle. `pointer-events-none` so it never intercepts a gate/operator click. |

## Found-by-hunt (non-empty — required)

| H | Row | Fix |
|---|-----|-----|
| **H1** (P1) | orphaned `pack_density_grid_r32` ran O(all resident chunks) every glyph recompose; no reader (T-174 glow leftover) | Removed the field + hot-path pack; free packer kept for the T-152.14 rung tests (recomputed on-demand in-test via `density_grid_of`). |
| **H2** (P2) | `residency.rs` computed then discarded `zoom_changed` (`let _ =`) | Removed the dead variable; the A5 glyph memo makes the unconditional refresh cheap. |
| **H3** (P1) | `world_host.rs::fetch_and_queue` had the **same** serial-await pattern as A2 (world residency chunks — slows zoom-out + boot) | Concurrent per-batch fetch (`join_all`), batch order preserved. |
| **H4** (P2) | duplicate `CONTOUR_RGBA` — frontend redeclared darker than core `vector_compose.rs:130` | Bumped only the live frontend const (A3); core oracle left for byte-parity. Logged. |
| **H5** (P1) | damage-mark audit under damage-driven render: `set_slot_drag_delta` (B3) **and** `set_slot_px_to_m` (zoom-during-drag) wrote uniforms without `damage.mark()` | `damage.mark()` on both. |
| **H6** (P2) | building fill sticky guard vs the tree bug | **Audited — no leak.** LOD-off buildings clear the pin set (→ `pin_settled` vacuously true → empty fill uploads); toggling off hides via `b_vis`. No sticky repro; no fix needed. |

## Perf — before/after

- **Encode floor (this build, SwiftShader headless bench):** CPU frame-encode **0.0236–0.125 ms → ~42,283 fps-equiv**; `S_bench_encode_60_floor` + `S_dup_fetches_zero` + `S_idle_fetches_zero` all true. Matches the T-173 ceiling (13k–42k fps-equiv) → the ≤1 ms / 1000-FPS potato floor is **intact, no regression**, and pan/zoom cause **zero** fetch/console storm.
- **Mechanism deltas vs T-174:** (a) island zoom-out no longer draws up to 150 k **stale** tree instances every frame (A1) — the dominant pan/zoom-at-island cost; (b) each post-zoom settle no longer pays the orphaned 625-alloc density pack (H1); (c) a fine in-band zoom is now a glyph **memo hit** instead of a full ≤150 k repack (A5); (d) forest/world chunk fetch is ~12× fewer serial RTTs (A2/H3).
- **On-GPU `rf` / `window.__editorBench(500)` before/after: operator G-A** (RTX 3070). SwiftShader is a software rasterizer (on-screen fps not representative — same caveat as T-173/T-174); the mechanism + zero-thrash + encode floor are verified headless, the felt stutter delta is the operator pass.

## Manual notes (operator G-A — drive `make api` + `make leptos`, release)

1. **Zoom-out unpacks trees** — zoom in (trees visible) → zoom out past the glyph band / to island: the packed trees are **gone** from the GPU (forest mass shown), not a frozen sticky set that only culls out of frame.
2. **Forest settle** — after a zoom-out, the forest mass fills in promptly/progressively (no long dead pause).
3. **Contours** — readable warm hairlines over both satellite and Map basemaps at default + mid zoom.
4. **Cold-load slots** — open a mission whose IDB snapshot has moved slots: icons are at the correct world positions on first paint (no wrong-offset flash).
5. **Place ghost** — drag an asset from the palette over the map: a translucent ring follows the cursor; release drops the slot there.
6. **Drag preview** — drag a selected slot: the preview tracks the pointer continuously; release commits (one undo step).
7. **Selection** — click / marquee selection tint is immediate under a large mission.
8. **Loading bar** — cold-open `/missions/:id/edit`: a loading overlay with a phase label + bar shows until the mission + map are ready (no silent half-ready map).

## Cursor doc list (post-ship doc sync — Claude did NOT edit docs)

- **CLAUDE.md** §Status: add **T-175** shipped bullet (MC interaction + LOD + pan/zoom perf); bump `latest shipped`.
- **`.ai/tickets/registry.json`**: T-175 → `shipped` @ tag/sha; `./scripts/ticket sync`.
- **`docs/platform/t175_mc_interaction_lod_perf.md`**: mark Acceptance A1–A5 / B1–B5 + C met; link this verify log.
- **`docs/website/frontend/pages/mission-editor.md`**: note the boot **loading overlay** (BootPhase), palette **place ghost**, live **drag preview**, and readable contours; `Live source:` `mission_editor.rs` / `world_assets/`.
- Optional: a MC ROADMAP note that CPU-side min-px sizing keeps a sub-`glyph_size_floor_zoom` repack (a GPU-uniform port would break byte/TS parity — inherent, not deferred).
