# T-172 inventory — Leptos SPA + Mission Creator bug bash (Phase 0, Class-R)

Recon date 2026-07-18. Sources: operator matrix (`docs/platform/t172_leptos_bug_bash.md`),
screens `01`–`05`, code recon of `apps/website/frontend/src/**`, `crates/map-engine-*`,
`tools/tbd-tools` gates, and React git history (`50bba633^` pre-deletion tree, T-154 commits
`a65529b3`/`3b3e4d16`).

## A — Platform shell (operator-seeded)

| ID | Sev | Bug | Root cause (file:line) |
|----|-----|-----|------------------------|
| A1 | P0 | User menu click does nothing | `layout.rs:142-153` avatar `<button>` no `on:click`; comment :96-97 "dropdown is a follow-up" |
| A2 | P1 | Sidebar active highlight stuck | `layout.rs:206` `use_location().pathname.get()` sampled once at body eval; static `a_class` :235-247 |
| A3 | P2 | Site-wide scroll lag | `style/aegis.css:114-121` `body { background-attachment: fixed }` breaks composited scrolling → every scroll repaints bg + re-blurs all ~20 `backdrop-blur-xl` surfaces |
| A4 | P0 | Wiki click doesn't change article | `wiki.rs:315` `let active = &MANUALS[0]`; rows never given `on_click` (`ListDetailItem` supports it: `split_pane.rs:122-163`) |
| A5 | P0 | Vehicle DB same dead selection | `vehicles.rs:112` `&VEHICLES[0]` static |
| A6 | P0 | Modpacks same dead selection | `modpacks.rs:138` `&MOCK_MODPACKS[0]` static |
| A7 | P1 | Dossier sheet: no loading bar, pops in | fetch starts only after Sheet mounts (`missions.rs:552-615`); fallback text-only :585-587 (`.animate-mc-load-bar` exists unused, `aegis.css:251`); Sheet renders no DOM while closed → `transition-transform` has no from-state (`ui.rs:183-195`) |
| A8 | P1 | Breadcrumb stale after SPA nav | `layout.rs:76` one-shot `pathname.get()`; frame choice :32 same pattern |
| A9 | P0 | Hamburger dead; no narrow-viewport nav | `layout.rs:163-176` no `on:click`; `Sidebar` `hidden lg:flex` :181; no drawer exists |

## B — Mission Creator (operator-seeded)

| ID | Sev | Bug | Root cause (file:line) |
|----|-----|-----|------------------------|
| B1 | P1 | Editor laggy (boot + interact) | boot: sync DEM decode+hillshade (`world_assets/mod.rs:206-207`), 12 sequential bootstrap passes :97-100, double manifest fetch, 10 MB grid clone per settle (`dem_vectors.rs:53`), identical forest mesh re-uploaded every pass (`forest_mass.rs:129`); interact: B8 |
| B2 | P0 | CUR shows X/Y only — no Z | `eden_chrome.rs:545-546` scope comment; cursor is `(f64,f64)` (`mission_editor.rs:53`, set :614); DEM meters raster decoded then dropped (`world_assets/mod.rs:198-222`); retained 1600² `DemVectorGrid` (8 m cells, 10.24 MB) is sampleable — `bilinear_sample` exists (`map-engine-core/src/dem/sample.rs:82`) |
| B3 | P0 | Forest = opaque green blobs | mesh baked at alpha 1.0 (`world_assets/forest_mass.rs:209` → `vector_compose.rs:234` α=255); `forest_fill_alpha(zoom)` (0.45/0.35/0.12/0) computed :138 but used only as visibility gate :157. Blend state fine (ALPHA_BLENDING, engine.rs:766); `set_lane_opacity` covers textured lanes only (engine.rs:4117-4137) |
| B4 | P0 | Placed slots invisible (selectable) | slot lane gated on `atlas_ready`; `ensure_slot_atlas(rgba,w,h,uv)` (engine.rs:3308) **never called by frontend** → `slots_bind_soa`/`set_selection` documented no-ops (`mission_editor.rs:429-432`, `editor_ops.rs:730`). OBJ/SEL counts come from the doc, not the GPU lane — that's why they work |
| B5 | P0 | Drop-place: slot not visible | same root cause as B4; `after_doc_change` already rebinds SoA (`mission_history.rs:194`) |
| B6 | P1 | Trees can't collapse | no expand state anywhere; `outliner.rs::flatten` :247 walks all nodes; `palette_rows` always renders kids (`eden_chrome.rs:435`); `CatalogNode.default_expanded` ignored (`asset_catalog.rs:42`) |
| B7 | P1 | No open-folder icon / guide lines | `single_row` fixed icons (`eden_chrome.rs:229`, folder :277); indent-only padding :215 |
| B8 | P1 | Selection highlight laggy | every selection → `refresh_docks` (`editor_ops.rs:683`) `.set()`s BOTH full node trees :706-707 → O(n) re-flatten per click; rows already have fine-grained `is_sel` closures (`eden_chrome.rs:284-287`) |
| B9 | P1 | Missing chrome vs screen 05 | gap table below |
| B10 | P0 | Arsenal 2D SVG, must be 3D doll | `arsenal.rs:332` SVG `paper_doll`; **DollEngine intact + already linked in the wasm bundle** (`crates/map-engine-render/src/doll3d.rs`; policy `map-engine-core/src/doll/mod.rs`) — zero callers. Operator scope answer: **full screen-04 layout** |

## B9 chrome-gap table (Leptos today vs React `50bba633^` / screen 05)

| Control (screen 05) | React behavior | Leptos today | T-172 action |
|---------------------|----------------|--------------|--------------|
| File/Edit/View/Mission/Environment menu bar | dead stub buttons ("(soon)") | absent | menu bar w/ dropdown items (Save/Export, Undo/Redo, Settings…) — strictly better than stubs, gate-safe (editor route not in v-suite) |
| Editable mission title in strip | inline input → `setTitle` + dirty dot | static text | editable input → `editor_ops::set_title` |
| Time scrubber + HH:MM + weather select | `range 0..1439` + select → `updateEnvironment` | only in Settings dialog | inline scrubber + select bound to same doc fields |
| History button | present, disabled | absent | disabled icon button |
| Save Version dialog | semver+notes, size estimate, amber >200 MB, `animate-mc-load-bar` | bare semver input | port dialog |
| Toolbelt Ruler / LoS | present, disabled | present, disabled | parity OK (no change) |
| CUR↔SEL swap | label flips when exactly 1 selected, shows slot x/y/z | CUR only | implement |
| CUR Z cell | DEM-fed, 3 dp, `—` off-map | absent | B2 |
| SZ readout | `estimateCompiledBytes` (≤20-slot sample avg × n + 2048) debounced 500 ms, `formatBytes` decimal | absent | new pure `mission_size.rs` |
| FACTIONS/VEHICLES/MARKERS tabs | tabs present; Vehicles/Markers stub panels | Factions-only dock | tabs + stub panels ("placement lands in T-070/T-069") |
| Asset Browser heading + search | real filter (T-055) | absent | `filter_catalog` (case-insensitive, keep-folder-on-descendant-match, force-expand matches) |
| OUTLINER header + bottom-left icon strip | 5 visual-only tabs (Hierarchy active) | absent | visual-only strip (parity-legal stubs) |
| Bottom-right debug readout | z + FPS (T-151 HUD; screen 05 shows pre-Deck-retirement extended form) | absent | `engine.stats()` @ ~1 Hz: z · chunks · glyphs · FPS |

## Found-by-hunt (H rows — all in T-172 scope)

| ID | Sev | Find | Lead |
|----|-----|------|------|
| H1 | P1 | SidebarNav samples auth role once — nav sections never react to login/bootstrap (admin section appears only after hard reload) | `layout.rs:203` |
| H2 | P2 | `push_composite` re-uploads an identical concatenated forest mesh on every drain pass (12× boot, 6× per camera settle) | `world_assets/forest_mass.rs:129` |
| H3 | P2 | 10 MB `DemVectorGrid` deep-cloned on every camera settle | `world_assets/dem_vectors.rs:53` |
| H4 | P2 | `manifest.json` fetched twice per bootstrap | `world_assets/mod.rs:203,227` |
| H5 | P1 | `engine.on_camera_changed()` never called — slot px sizing + cluster gate go stale on zoom once the atlas exists | `mission_editor.rs` (wheel/pan/set_view paths) |
| H6 | P2 | Dialog/AttributesModal/MissionSettings/FactionManager pop in with no enter animation (same no-DOM-while-closed gap as A7) | `ui.rs:119-131` |
| H7 | P1 | Wiki READ/EDIT toggle dead (both buttons no handler) | `wiki.rs:403-414` |
| H8 | P1 | Modpacks "[ Launch Game & Auto-Download ]" dead | `modpacks.rs:239-244` |
| H9 | P0 | Login page "Sign in with Discord" button has no `on:click` — real OAuth start exists (`GET /api/v1/auth/discord/login`) | `app_routes.rs:61-66` |
| H10 | P1 | Arsenal missing page-doc-pinned controls: Download loadout JSON (M5.26b), COMPAT ACTIVE / LOADOUT VALID badges | `arsenal.rs` (folded into B10 full-layout rebuild) |
| H11 | P1 | `/wiki/:slug` routed but param ignored — deep links always render `MANUALS[0]` | `wiki.rs:313-326` |

Hunt methods run: stub-marker grep (follow-up/later slice/gate scope/stub/TODO/dead buttons/one-shot
`pathname.get()`/static `&X[0]`), full route table walk (code-level; live click-through lands in
Phase 4 sweep), React-git + page-doc parity diff, gate-internals read. Live browser + console/network
hunt continues during implementation — new finds append here as H12+.

## Perf suspects (ranked)

1. `body` `background-attachment: fixed` (site-wide scroll) — A3.
2. Selection → full dock tree rebuild — B8.
3. Boot: sync DEM decode + hillshade, sequential passes, double manifest fetch, forest re-uploads, grid clones — B1/H2/H3/H4.
4. `backdrop-blur-xl` count (~20 surfaces) — acceptable once (1) lands; not touched unless still laggy.

## Arsenal 3D port plan (B10)

Reuse intact wgpu `DollEngine` (`doll3d.rs`: create/resize/rotate/set_hover/anchor_px/set_states/
pick_region/render; region contract `REGION_KEYS` 14 == `arsenal_rules.rs::RAIL_REGIONS` 14).
New `arsenal_doll.rs` Leptos mount mirroring deleted `SoldierModel3D.tsx` (`3b3e4d16`): rAF render +
DOM callout chip/leader from `anchor_px`, drag>4 px rotate, hover pick, click select, `set_states`
effect, SVG `paper_doll` as create-error fallback. `arsenal.rs` rebuilt to screen-04 layout (icon
rail · filtered item list · doll · compat panel · badges · Download JSON) on the unchanged
picks/compat/registry/persist data flow. No three.js.

## Severity summary

P0 × 9 (A1, A4, A5, A6, A9, B2, B3, B4, B5, B10 → 10 counting B5 shared-cause) · P1 × 12 · P2 × 6.

## Gate notes (execution constraints)

- v-suite = frozen byte-exact structural DOM diff, 25 shell routes, `#root>:first-child`, listeners
  not serialized → all shell fixes must keep captured-state DOM byte-identical; overlays render no
  DOM while closed.
- Editor smokes: keep `[title="Cursor X"/"Cursor Y"]` + exact values, aside
  "Factions"/"ORBAT"/"Editor Layers" textContent, aria-labels, outliner default-expanded,
  `forest_polygons > 0`. Deliberate edits only: `arsenal`, `outliner-palette`, `doc`.
