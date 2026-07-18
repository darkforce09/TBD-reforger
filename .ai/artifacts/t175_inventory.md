# T-175 — Phase 0 inventory (MC interaction + LOD + pan/zoom perf)

Recon: 3 Explore agents + 1 Plan agent + direct reads. File:line-precise root causes for the
operator matrix (A1–A5, B1–B5) + a mandatory experience hunt (H-rows). All rows are **in scope**
and fixed in T-175 (no silent deferrals). B5 confirmed in-scope by operator ("Include B5").

Baseline: **T-174 @ `bbb99526`** (sat progressive, density-heatmap glow removed, dock guides).

## A — LOD / world layers

### A1 (P0) — tree glyphs sticky on zoom-out
Two independent leaks; both fixed.
1. **Host skip** — `apps/website/frontend/src/world_assets/world_host.rs:458-465` (`push_to_engine`):
   `if !trees.is_empty() || pin_settled { e.upload_icon_lane(0,&trees,true) }` (same props `:461`,
   badges `:464`). `pin_settled = residency.pin_settled()` (`residency.rs:1569-1573`, derived: all
   pinned chunks resident). On zoom-out the LOD clears trees (heatmap rung / below band → empty vec)
   but the enlarged island pin set keeps `pin_settled == false` → the empty upload is skipped → GPU
   retains the last zoomed-in tree buffer. Frustum cull only hides off-screen ones ⇒ "sticky set from
   the last zoom-in; only out-of-frame culled." pin_settled may never flip true during interaction, so
   it never self-heals.
   - **Repro:** open a forested mission, zoom in (z≈2, trees pack) → zoom out to island (z<0). Old
     trees remain on the GPU instead of handing off to forest mass.
2. **Engine skip** — `crates/map-engine-render/src/engine.rs:3251-3262` (`upload_icon_lane` empty
   branch): on `bytes.is_empty()` + `visible=true`, only the `LaneRole::WorldTrees` compute-cull
   buffer (WebGPU) is cleared; props/badges (kind 1/2) and **WebGL IconInstanced trees** early-return
   before `remove_lane` (only runs on `!visible`) → stale lane persists. Second leak; hits the WebGL
   path (editor smokes force WebGL2) and any non-cull lane.
- **Fix:** engine clears the lane's IconInstanced batch on empty regardless of `visible`; residency
  exposes `glyph_lane_lod_off(lane)`; host guard adds `|| glyph_lane_lod_off(lane)` so an
  LOD-off empty clears while a *want-but-still-loading* empty stays sticky (no mid-hydration flicker).
  Same lod-off gate applied to the building fill guard (`world_host.rs:428-429 skip_buildings`).

### A2 (P1) — forest mass delay after zoom-out too long
`apps/website/frontend/src/world_assets/forest_mass.rs:128-138` (`fetch_missing`): despite the
`FETCH_CONCURRENCY` name it awaits every `.bin` **sequentially** (`for batch in ids.chunks(N) { for
id in batch { fetch_bytes(id).await } }`). Zoom-out to island needs a large new chunk set → dozens of
serial RTTs in one `run_viewport` pass before a single `push_composite`. The `present`-count memo
(`:165-172`) already supports progressive partial fill across passes, so the bottleneck is purely the
serial fetch.
- **Fix:** fetch each batch concurrently (`join_all`) + push after each batch (progressive fill).

### A3 (P1) — contour lines too dark
`apps/website/frontend/src/world_assets/dem_vectors.rs:20 CONTOUR_RGBA = [90,70,40,180]` (luma ~72,
α180), only consumer `:114` (`compose_contour_hairlines` → flat rgba on every vertex). Contours are
native 1px `LineList` (`engine.rs:519`) — wgpu cannot widen a line, so color/alpha is the only lever.
No major/minor tiers. Draw order 5 over Satellite(1)/Sea/Hillshade/Landcover → the dark hairline sits
over both a dark sat photo and the tan Map basemap = near-invisible.
- **Fix:** bump to a readable warm tan-brown at higher alpha (`[188,150,100,235]`), operator-tunable;
  optional index-contour tiers if level info is cheap.

### A4 (P1) / A5 (P0) — pan / zoom stutter
`crates/map-engine-core/src/world/residency.rs:1091-1106` `compose_key` hashes raw
`deck_zoom.to_bits()`, so any zoom delta busts the glyph memo → full `rebuild_glyph_buffers`
(≤150k instances) on every post-zoom settle; it also runs the orphaned `pack_density_grid_r32`
(H1). Pan at constant zoom already memo-hits (draw-set stable) once A1's stale-lane churn is gone.
The pack's zoom dependence is: discrete `class_visible` bands + `heatmap_trees` (→`pack_trees`) +
per-instance importance `z >= importanceZoom` (continuous, T-152.21) + **min-px size**
(`size_with_min_px = max(size_m, min_px·2^−z)`, continuous while the floor dominates) + strip width.
- **Fix:** floor-aware band-signature memo (`floor_term(z, Z_lane)=sentinel when z≥Z_lane else
  z.to_bits()`), split into tree/propbadge/strip keys; move `heatmap_trees` compute above the memo;
  static importance-breakpoint activation index. Memoizes the dominant `z∈[1,3)` tree pack; the
  sub-`Z_lane` repack is inherent to CPU min-px sizing (GPU-uniform port would break byte-parity —
  architectural boundary, not a deferral; the dominant stutter IS removed).

## B — interaction / document

### B1 (P0) — first-load slots at wrong position
`apps/website/frontend/src/mission_editor.rs`: engine task (`:439-526`, first bind `slots_bind_soa`
`:490-494`) and doc task (`:365-437`) run as independent `spawn_local`s; the engine task does not
await doc restore. The IDB restore swap (`:375-382`) replaces the doc then calls only `refresh_hud()`
(`:389`) — **not** a rebind. If engine-create wins the race, `:494` binds the seed SoA; restored
positions never reach the GPU until a manual edit. Warm-boot hydrate "adopted==server" returns early
w/o rebind (`mission_hydrate.rs:92-93`); non-UUID smoke route returns immediately (`:48-49`).
- **Repro:** cold-load a mission whose IDB snapshot has moved slots → glyphs at seed positions until
  an edit refreshes.
- **Fix:** `mission_history::rebind_engine_from_doc()` + two-Cell handshake (`restore_settled`,
  `engine_mounted`) → single bind from the settled doc, no flash; set `restore_settled` on the
  no-blob path too.

### B2 (P1) — palette drag: no place ghost
`editor_ops.rs:817-822 begin_place` only stashes `pending`; the pointermove handler
(`mission_editor.rs:640`) never references `pending` (only the CUR text readout `:664-690`). The slot
first materializes on pointerup (`place_at`→`add_slot`→`after_local_edit`→`slots_bind_soa`). No engine
preview call exists.
- **Fix:** engine `LaneRole::SlotPlacePreview` single translucent glyph + `set_place_preview`/
  `clear_place_preview`; FE draws it under the cursor during a pending place.

### B3 (P0) — slot drag preview ~1px then teleport on release
NOT units (verified world-consistent end-to-end: `select_tool.rs:191-197` world m →
`set_slot_drag_delta` "world meters" `engine.rs:3793-3807` → `shader.wgsl:175` adds to world pos),
and the delta is recomputed per pointermove (`mission_editor.rs:771-773`). Root cause = the engine is
damage-driven (`set_continuous_render(false)` `:462`; `render()` no-ops without damage
`engine.rs:1815-1820`). Start phase marks damage (one paint at the 4px threshold); the **Delta** phase
(`set_slot_drag_delta`) writes the uniform via `queue.write_buffer` but **never** `self.damage.mark()`
→ no repaint until the pointerup commit → preview frozen ~1 threshold-hop then "teleports."
- **Fix:** `damage.mark()` in `set_slot_drag_delta` (+ `set_slot_px_to_m` for zoom-during-drag).

### B4 (P1) — selection / marquee tint laggy
`engine.rs:3450-3464 set_selection` → full O(n) `rematerialize_slot_lane` (`:3717-3738`,
`pack_slot_instances` over all slots + re-upload) on every click / outliner select / marquee-release /
paste. Marquee **drag** uses a separate `upload_marquee` lane (`:796`), not per-move re-packing
(already OK). The tint is inline in the 20B slot record (`size`@+8, `tint`@+16, `slots_gpu.rs:80-97`).
- **Fix:** O(delta) tint patch via the existing `patch_slot_lane` 12B block writes; full rematerialize
  only for cluster/selection-only + drag.

### B5 (P1) — no boot loading UX
`mission_editor.rs:1132-1218` editor view = canvas + chrome + modals only; zero loading overlay
during catalog fetch → persist/hydrate (`:365-437`) → engine-create/atlas/slots-bind (`:439-526`) →
world bootstrap (residency drain ≤12 passes `world_assets/mod.rs:168-269`). Only reactive load signal
is `catalog` (palette); `persist_ready`/`persist_loaded` are non-reactive `Rc<Cell>`; no signal for
engine/atlas/world-settle. The React T-060 determinate overlay was not ported (only the
`.animate-mc-load-bar` CSS remnant survives, `aegis.css:261/265`).
- **Fix:** reactive `BootPhase` threaded from the async tasks + a full-bleed overlay (phase label +
  bar) covering cold hydrate + initial map/world settle; lighter on warm return.

## C — experience hunt (mandatory; non-empty)

- **H1 (P1)** — orphaned `pack_density_grid_r32` (`residency.rs:1144-1151`) runs O(all resident
  chunks) on **every** glyph recompose; no public getter, no GPU lane, only unit tests read
  `density_grid` (T-174 glow leftover). **Fix:** make it lazy/on-demand (compute in the test/telemetry
  path) so the zoom hot path stops paying it; keep the fn for the T-152.14 rung tests.
- **H2 (P2)** — `residency.rs:576` computes then discards `zoom_changed` (`let _ = zoom_changed;`);
  the `!world_want` branch always runs `refresh_draw_set_and_glyphs`. **Fix:** review/remove; ensure
  no redundant recompose.
- **H3 (P1)** — `world_host.rs:356-360` residency chunk fetch (`fetch_and_queue`) has the **same**
  sequential-await pattern as A2 ("Sequential batches of FETCH_CONCURRENCY"), `FETCH_CONCURRENCY=12`.
  Slows A5 zoom-out (world chunks) + B5 boot (residency drain). **Fix:** concurrent per-batch fetch.
- **H4 (P2)** — duplicate `CONTOUR_RGBA`: frontend `dem_vectors.rs:20` redeclares a darker value than
  the core canonical `vector_compose.rs:130 [120,96,64,200]`. Frontend is the live path (folded into
  A3); core left alone (oracle/parity). **Fix:** log; bump only the live frontend const.
- **H5 (P1)** — damage-mark audit under damage-driven rendering: `set_slot_drag_delta` (B3) and
  `set_slot_px_to_m` (`engine.rs:3774-3788`, zoom-during-drag) write uniforms with no
  `self.damage.mark()`. **Fix:** mark damage on both (B3 + sibling).
- **H6 (P2) — AUDITED, no leak (no fix needed).** The building fill guard `world_host.rs:428-429
  skip_buildings` shares the `pin_settled` shape, but when buildings are LOD-off (`world_want` false
  → below `BUILDING_MIN_ZOOM` and below `min_importance_zoom`, `residency.rs:624-628`) the **pin set
  is cleared**, so `pin_settled` is vacuously true and the empty fill uploads → buildings clear. The
  tree bug differs: at island zoom the tree pin set stays non-empty (world chunks pinned for the
  badge/importance band) so `pin_settled` is false while the tree LOD says off — which A1 fixes.
  Toggling buildings off hides via `b_vis` (visible=false), not the skip. Verified by reading the
  clear paths; no sticky building repro exists.

**Method:** hunt continues while fixing; any new row is added here and fixed before the T-175 tag.
