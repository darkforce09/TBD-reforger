# T-174 — Phase 0 inventory (MC sat / heatmap / dock guides)

Recon: 3 Explore agents + direct reads. File:line-precise root causes for S1–S3. Operator
override during plan review: **"Remove the heatmap, it's not something I want"** → S2 is a full
removal of the density-heatmap glow (no toggle), keeping the load-bearing over-budget LOD signal.

## S1 — low-res satellite (localhost preview-stuck)

Load path (`apps/website/frontend/src/world_assets/satellite.rs`):

- `load_satellite` ran `try_preview` (coarse Range mip, `PREVIEW_MAX_EDGE = 1024`) then, unless
  gated, `load_unified_full` (full 14-mip 152 MB chain). **Preview→full progressive already
  existed** and prod already used it.
- Gate (old `:402`): `if sat_preview_only() || sat_dev_preview_default() { return; }`.
- `sat_dev_preview_default()` (old `:409-422`) returned `true` on `localhost` / `127.0.0.1` /
  `*.localhost` unless `?sat=full` → **localhost early-returned after preview → stuck on the
  ≤1024 px blur**. That branch (added for a "don't freeze the tab on 152 MB" dev convenience) was
  the entire root cause.
- Full bundle present + served locally: `packages/map-assets/everon/satellite/everon-sat.tbd-sat`
  = 152,713,114 B, manifest `mipCount = 14`. Served via API `/map-assets` (`ServeDir`, Range-aware)
  and the Trunk `/map-assets` proxy → `:8080`.
- **Gate dependency to preserve:** `tools/tbd-tools/src/smokes.rs` — the 18-smoke `EDITOR_SUITE`
  default path + `smoke_fullmap` + `smoke_hillshade` all use `?sat=preview`; `smoke_fullmap`
  asserts `satMode==='single'` + **zero** full-bundle GET (`SAT_FULL_BYTES = 152_713_114`).
  `sat_preview_only()` (`sat=preview` in `location.search`, hostname-independent) still short-circuits
  the full load → gates unaffected.

Load path (A/B): **A = preview-only** (`sat_preview_only()` OR the removed localhost default);
**B = preview→full progressive** (everything else). Fix = make localhost take path B by dropping
the localhost default; path A remains for `?sat=preview` (CI/gate + fast local iteration).

## S2 — density-heatmap green glow (auto over-budget rung; removed)

Not a user toggle — an automatic LOD "over-budget rung":

- `crates/map-engine-core/src/world/residency.rs` field `heatmap_trees` (`:237`, init `false`
  `:313`) flips **on** when the visible tree census > `INSTANCE_BUDGET = 150_000`
  (`density_ladder.rs:59` predicate; hysteresis enters at 150k, exits below 127.5k, `:1155-1170`).
  At island zoom (`deck_zoom < 0`) the whole forest is in view → census ≫ 150k → rung on. No hard
  zoom threshold; the zoom relationship is emergent.
- Glow upload (host): `world_host.rs` `push_to_engine` — `density_vis =
  heatmap_trees_active()` (old `:451`) → `e.upload_density_grid(…, density_vis)` (old `:478`).
- Engine lane: `map-engine-render/src/engine.rs::upload_density_grid` (old `:3326-3424`) built a
  full-terrain quad at **alpha 0.85**, green texel encoding `density_heat.rs::density_counts_to_rgba`
  (`R=30, G=40..220, B=50`), draw lane `LaneRole::DensityHeat` (`draw_order.rs`, order 14).
- **Which zoom bands lit the glow:** any zoom where the in-view census exceeds 150k — i.e.
  island/coarse (`z<0`) with dense forest. Zoomed-in local (`z≥0`) drops below budget → off.

**Coupling caveat (load-bearing):** `heatmap_trees` field also (a) suppresses tree glyphs
over-budget (`:1208` `pack_trees = tree_want && !heatmap_trees`) and (b) keeps forest mass alive
(`:1452` `forest_fill_effective`). The LOD boolean derives from `visible_tree_count`, **not** the
density grid. ⇒ removing the glow render path leaves the over-budget LOD intact: island zoom shows
forest-mass fill, no glyphs, **no glow** (operator's wanted look; spec allows forest mass to stay).

Removal set (glow render path only; ref-grep-confirmed reachable only via the glow):
frontend host upload + `heatmap_trees` bridge stat; engine `upload_density_grid` +
`density_heatmap` field/stats + `density_heat.rs` + `LaneRole::DensityHeat`; wasm
`density_grid_r32_bytes`/`density_grid_size` exports; core getters
`density_grid_r32_bytes`/`density_grid_dims`. **Kept:** `heatmap_trees` LOD signal + the
`density_grid` count field/pack (exercised by the T-152.14 LOD-rung tests — internal accounting,
renders nothing) + the residency `stats_json` `heatmap_trees` telemetry key (reports the LOD rung).

## S3 — full-dock-height guide rails

`apps/website/frontend/src/eden_chrome.rs` `guide_spans` (`:646`) emits per-depth
`absolute inset-y-0 w-px bg-white/25` stems. `inset-y-0` spans the nearest **positioned** ancestor.

- `ROW` / `ROW_ACTIVE` recipes are `relative` (`:636-637`) → for slot / outliner-folder / palette-leaf
  `<button>` hosts the guide correctly spans one row.
- **Escaping hosts** are plain `position: static` `<div>`s: `single_row` Unfiled (`:728`), Faction
  (`:737`), Squad (`:746`), and `palette_rows` folder (`:958`). For these, `inset-y-0` skips every
  static wrapper (`<aside>`, `mt-1`, scroll/overflow divs) and resolves against the
  **absolutely-positioned full-dock-height dock wrapper** (`mission_editor.rs:1169` left / `:1177`
  right) → one top-to-bottom rail per depth column, coexisting with the correct short stems.
- **Virtual spacers ruled out:** the rail is present in the non-virtualized `<50`-row branch too
  (`VIRTUAL_SLOT_THRESHOLD = 50`, `outliner.rs:39`); spacers only change scroll geometry, not the
  guide's containing block.

Root cause = missing `relative` on the header/folder row hosts. Fix = prepend `relative` to those
4 `<div>` class strings (each row becomes its own positioning parent; `inset-y-0` clips to the row).
