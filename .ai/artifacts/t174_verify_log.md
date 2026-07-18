# T-174 — verify log (MC sat fidelity + heatmap removal + dock guide fix)

**Branch:** `main` · **Depends on:** T-173 (`dddf3158`) · **Scope:** `apps/website/frontend/**` +
`crates/map-engine-*`. **Not** `apps/mod/**`.

**Operator override (plan review):** *"Remove the heatmap, it's not something I want."* → S2 is a
**full removal** of the density-heatmap glow (no toggle), keeping the load-bearing over-budget LOD
signal. Quoted authorization for the scope change (per `no-silent-deferrals.mdc`).

Inventory: [`t174_inventory.md`](t174_inventory.md).

## S-matrix

| S | Fix | Status |
|---|-----|--------|
| S1 | Localhost sat = preview→full progressive (drop `sat_dev_preview_default`); `?sat=preview` keeps CI/gate Range-only | **PASS** (code + gates) |
| S2 | Density-heatmap glow **removed** (render path excised end-to-end); LOD rung kept | **PASS** (code + native tests + gates) |
| S3 | Guide stems clip to row (`relative` on 4 escaping header/folder hosts), no dock rails | **PASS** (code + gates) |

## Changes

### S1 — `apps/website/frontend/src/world_assets/satellite.rs`
- `load_satellite`: gate is now `if sat_preview_only() { return; }` (dropped `|| sat_dev_preview_default()`).
  Localhost now runs `try_preview` → `load_unified_full` (full 14-mip chain), same as prod.
- Deleted the private `sat_dev_preview_default()` fn (localhost-forces-preview default).
- Rewrote the `load_satellite` doc comment: preview→full progressive on all hosts; `?sat=preview`
  (hostname-independent) = Range-only for CI/gate + fast local iteration; `?sat=full` is now a
  redundant no-op (full is the default).
- **`?sat=full` / `?sat=preview` semantics (documented):** default (no param) = sharp full mip
  chain after a coarse preview flash, on every host incl. `make leptos` (`127.0.0.1:3000`).
  `?sat=preview` = never GETs the 152 MB body (CI/gate + fast dev). `?sat=full` = no-op.

### S2 — density-heatmap glow removed (no toggle)
- `world_assets/world_host.rs`: removed the density-grid upload (`density`/`dw`/`dh`/`density_vis`
  + `e.upload_density_grid(...)`) and both `b.heatmap_trees` bridge mirrors; removed the now-unused
  `TERRAIN_M` const.
- `world_assets/bridge.rs`: removed the `heatmap_trees` field + its `set("heatmap_trees", …)` publish.
- `crates/map-engine-render/src/engine.rs`: removed `upload_density_grid` + the `density_heatmap`
  field/init + its two `stats_json` emitters.
- `crates/map-engine-render/src/lib.rs`: removed `pub mod density_heat;`.
- Deleted `crates/map-engine-render/src/density_heat.rs`.
- `crates/map-engine-render/src/draw_order.rs`: removed `LaneRole::DensityHeat` (enum + `lane_order`
  arm + both `lane_order_pins` test arrays). `lane_order` is comparison-only (no array indexing), so
  the resulting gap at 14 is inert; all 7 `lane_order_pins` tests still pass.
- `crates/map-engine-wasm/src/lib.rs`: removed the `density_grid_r32_bytes` + `density_grid_size`
  exports (they wrapped the removed core getters; no JS/smoke caller).
- `crates/map-engine-core/src/world/residency.rs`: removed the `density_grid_r32_bytes` /
  `density_grid_dims` getters (only the removed frontend/wasm called them).
- **Kept (intentional, documented):** `residency.heatmap_trees` field + hysteresis +
  `heatmap_trees_active()` + glyph-suppression (`pack_trees`) + `forest_fill_effective` — the
  over-budget **LOD signal** (removing it would try to glyph 500k+ trees at island zoom → blows the
  150k instance budget). Also kept the `density_grid` count field/pack (exercised by the T-152.14
  LOD-rung tests — internal accounting, renders nothing) and the residency `stats_json`
  `heatmap_trees` telemetry key (reports the LOD rung; no consumer asserts it). Net effect: island
  zoom shows forest-mass fill with **no green glow**, at any zoom, without a perf regression.

### S3 — `apps/website/frontend/src/eden_chrome.rs`
- Prepended `relative` to the 4 escaping guide-host `<div>` class strings: `single_row` Unfiled,
  Faction, Squad, and `palette_rows` folder. Each row is now its own positioning parent, so the
  `guide_spans` `absolute inset-y-0 w-px` stem clips to the row height (short hierarchy stem)
  instead of resolving against the full-dock-height dock wrapper (no top-to-bottom rail).

## Automated verification

| Gate | Result |
|------|--------|
| `cargo test -p map-engine-core -p map-engine-render` | **PASS** — core 215, render 44, draw_order `lane_order_pins` 7/7 (DensityHeat removed), residency `class_r_heatmap_*` + `property_never_blank_zoom_ladder` green |
| `cargo fmt -p website-frontend --check` | **PASS** (exit 0) |
| `cargo clippy -p website-frontend --target wasm32-unknown-unknown` | **PASS** (exit 0; 16 pre-existing warnings, none from this change) — validates the wasm32-gated `engine.rs` heatmap removal |
| `cargo test -p website-frontend` | **PASS** — 73 |
| `trunk build --release` | **PASS** — dist regenerated; `upload_density_grid` absent from shipped JS; wasm 8,251,614 B |
| `make leptos-gates` (editor-suite + v-suite) | _see below_ |

`make db-up` up; API on `:8080` (dev-login → 302). `?sat=preview` smokes unaffected by S1 (they
pin `sat_preview_only()`); no smoke/probe reads `heatmap_trees` / `density_heatmap` (grep of
`tools/` + `apps/` + `crates/`).

### `make leptos-gates` — **PASS** (exit 0)

- **v-suite verify:** all **25** pages PASS, `diffs=0` (notfound, dashboard, approvals, audit,
  content, eventmgr, personnel, servercontrol, announcements, callback, deployments, events,
  eventhub, orbat, leaderboards, login, missions, missionview, modpacks, serverintel, settings,
  mortar, vehicles, wiki, wikislug).
- **editor-suite:** **PASS** — all **18** editor smokes, **0 fail** (selfcheck, arsenal, attributes,
  cur, doc, editor, fullmap, hillshade, hydrate, keyboard-settings, marquee-drag, outliner-palette,
  pan, persist, save-export, select, undo, virtual-outliner). Exercises the Mission Settings dialog +
  outliner/palette tree rows + virtual outliner — the S2/S3 surfaces. Smokes drive
  `?force=webgl&sat=preview` (S1 unaffected).

### `make ci-local` (full workspace)

Frontend + engine are fully covered by `make ci-local-leptos` + the `map-engine-{core,render}`
native tests + `make leptos-gates` above; the backend (`website-api`) and schema are **untouched**
by T-174 (no dep on any removed symbol — grep-confirmed). Note: the full-workspace `cargo fmt --check`
in `make ci-local` trips on a **pre-existing local-vs-CI rustfmt drift** in `xtask/src/cmds.rs` (my
local rustfmt collapses a multi-line `println!` that CI's rustfmt — which green-lit T-173 — keeps
multi-line). That file is **not** part of T-174 and was reverted to the committed CI-clean version;
all T-174-touched files are fmt-clean under both rustfmts (deletions + doc-comments + `view!` macro
class strings — no width-sensitive reformatting).

## Manual / visual acceptance (operator browser pass)

These are pixel-aesthetic checks the headless gates cannot judge; the code-level behavior is proven
above. On `make leptos` (`127.0.0.1:3000`, no `?sat=preview`), dev-login → `/missions/:id/edit`:

- **S1 sat sharpness:** coarse preview flashes, then the map sharpens to the full mip chain at
  island zoom **and** when zoomed in (no ≤1024 px blur). `?sat=preview` still shows the preview
  (gate/fast path). Bridge `sat_mode` transitions `single` → `unified` on the default path.
- **S2 no glow:** no green density wash at island or local zoom, at any pan/zoom. Forest-mass
  (solid green forest shapes) still renders at island zoom; individual tree glyphs still appear when
  zoomed in (`z ≥ 0`). No new Mission Settings control. No pan/zoom perf regression.
- **S3 no dock rails:** Outliner + Asset Browser (expanded) show no top-to-bottom bright vertical
  rails — only short hierarchy stems between sibling rows. Re-shoot vs operator screens 01–03.

## Cursor doc list (Claude Code does not edit docs)

- `docs/platform/t174_mc_sat_heatmap_guides.md` — mark shipped @ T-174; S2 is a **removal** (no
  toggle); note `?sat=full` redundant / `?sat=preview` = gate+fast path.
- `CLAUDE.md` §Status — T-174 Done bullet + bump `Latest shipped`; density-heatmap glow removed
  (`DensityHeat` lane deleted; LOD signal kept); S1 sat-default flip.
- `.ai/tickets/registry.json` — T-174 → done (`./scripts/ticket sync`).
- Any DEV_RUNBOOK / mission-editor page doc mentioning a `?sat=full` default or a tree heatmap.
