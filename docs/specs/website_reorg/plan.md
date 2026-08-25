# T-934 — Website Reorganization Program (audit.md §3 + §4)

**Source:** `apps/website/audit.md` §3 (folder restructure + monolith decomposition + backend nesting) and §4 (operator-added `MissionEditorPage` 2-phase resolution).
**Scope lock:** reorg only. Audit §2 bug/perf findings are a separate future program.
**Operator decisions (2026-08-25):** land ASAP (conflicts with in-flight T-090/factory accepted); executor claude-code; one child per commit, direct to `main`, tagged `T-934.x`.

## 1. Audit corrections (ground truth, verified against live code)

1. **Monoliths are half test code.** `mission_editor.rs` production ends ~line 7,498; ~7.9k LOC is `#[cfg(test)]`. `arsenal.rs` production ends ~2,800; tests include `class_r_scrub`, called cross-module (`crate::arsenal::class_r_scrub::live_code` from `event_hub.rs`, `event_manager.rs`, `mission_overview.rs`, …). `editor_ops.rs` is 100 % production.
2. **439 `include_str!` sites** across 44 frontend files (Class-R/S source guards). Self-includes (`include_str!("own_name.rs")`) survive *moves* (path is relative to the file's own directory) and break only on *renames*. Cross-file includes (~90 sites, inventory §5) break whenever relative geometry changes. Native `cargo test` (inside `ci-local-leptos`) detects both classes loudly.
3. **Audit §3.2/§3.3 split targets partly exist already** (`arsenal_rules.rs`, `asset_catalog.rs`, `arsenal_doll.rs`, `mission_history.rs`, `mission_hydrate.rs`, `eden_tree.rs`, `editor_session.rs`, `mission_commands.rs`, `mission_doc.rs`, `yrs_persist.rs`) — those only move; the audit's LOC estimates double-count them.
4. **No frontend `lib.rs`** — `main.rs` is the module root (73 `mod` decls, ~15 wasm32-cfg-gated). `main.rs` stays root; folder `mod.rs` files must reproduce each `#[cfg(target_arch = "wasm32")]` gate exactly.
5. **Files audit table 3.5 omits** (explicit placements, no silent drops): see §4 migration table rows marked `(omitted in audit)`.
6. **Backend §3.4 rejected as written:** would collapse 22 handler files into 5 new monoliths, omits 12 handlers, names a nonexistent `main.rs` (entrypoint is `src/bin/api.rs`), and moving `app.rs` would break the `personnel.rs`/`content.rs` cross-crate `include_str!` pins. Corrected layout in §3 Phase C keeps current file granularity nested by domain. `services/` stays flat — explicit decision (14 small cross-domain files).
7. **External path pins** to update atomically with their targets: `xtask/src/gate_t180.rs` (`EDITOR_OPS`/`EDEN_CHROME`/`ORBAT_MGR` consts), `xtask/src/gate_t439.rs` (`FE_REL` → asset_catalog), `xtask/src/ai.rs:438`, `xtask/src/gate_route_tags.rs` (handlers/servers.rs), `xtask/src/migrate_v2.rs` owns-inference tests. Verified safe: Tailwind (`style/aegis.css` uses recursive `@source "../src/**/*.rs"`), `Trunk.toml`, `index.html`, `tools/tbd-tools/src/sroutes.rs` (`router.rs` stays at root).

## 2. Program structure

16 children, ~16 commits. Dependency chain:
A: .1 → {.2, .3, .4} → .5 → .6 · B: .6 → {.7, .8, .9}; .9 → .10 → .11 → .12 · B2: .12 → .13 → .14 · C: .15 → .16 (C independent of B/B2, may interleave).

Per-move-child checklist (Phase A):
1. `git mv` batch; 2. folder `mod.rs` (cfg gates preserved); 3. crate-wide `crate::x` → `crate::new::path::x` rewrite; 4. self-guard renames + cross-file `include_str!` repoints (fixtures gain `../` per directory level); 5. `grep -rn "frontend/src/" xtask/ tools/` pin sweep; 6. `cargo xtask mk ci-local-leptos`.

## 3. Children

### Phase A — frontend mechanical moves (content byte-identical)

- **T-934.1 — core/ + shell/**: `auth, client, dto, sse, datefmt, toast, ui, url_guard, split_pane` → `src/core/`; `layout.rs` → `shell/layout.rs`; `nav.rs` → `shell/nav_config.rs`. `router.rs`/`app_routes.rs`/`main.rs` stay at root. No fabricated `topnav/sidebar/mobile_drawer` (that is a `layout.rs` split — out of program scope, recorded here, not silently dropped). dto R-api golden fixture paths gain `../`. Cross-guards: `main.rs→sse.rs`, `content.rs→client.rs`, `ui.rs→orbat_manager.rs`, `sse.rs→server_intel.rs`.
- **T-934.2 — pages/public/**: `dashboard, announcements, server_intel, wiki, vehicles, modpacks, mortar, settings, deployments, leaderboards`. Repoint `core/sse.rs` guard → `../pages/public/server_intel.rs`; `server_intel.rs` fixture path.
- **T-934.3 — pages/operations/ + pages/admin/**: `events`→`operations/event_schedule.rs` (self-guards renamed), `event_hub, orbat_selection, orbat_manager, faction_manager` → `operations/`; `event_manager, approvals, server_control, personnel, content, audit` → `admin/`. `orbat_manager` keeps audit placement; its guards to editor files become longer relatives. `personnel.rs`/`content.rs` `CARGO_MANIFEST_DIR`-rooted includes of `api/src/app.rs` unaffected.
- **T-934.4 — editor/ library + tools + world**: `missions.rs`→`editor/library/mission_library.rs` (self-guards renamed; `mission_overview.rs` cross-guard repointed), `mission_overview.rs`, `create_mission_dialog.rs`→`library/create_dialog.rs` (self-guards renamed); `select_tool, ruler_tool, los_tool, place_helpers` → `editor/tools/`; `world_assets/` (9 files), `world_layer_prefs.rs`, `mission_size.rs` → `editor/`. Fix `#[path]` remount of `tbd_sat.rs` and world_assets guards in still-flat `mission_editor.rs`; `missions.rs → ../Cargo.toml` include gains `../`.
- **T-934.5 — editor/panels/**: `eden_dock_left→dock_left`, `eden_dock_right→dock_right`, `eden_top_strip→top_strip`, `eden_toolbelt→toolbelt`, `eden_tree→outliner_tree`, `attributes→attributes_modal`, `validation_panel`, `eden_zones→zones_panel`, `eden_vehicles_panel→vehicles_panel`, `eden_settings→settings_modal`, `eden_help→help_modal`, `context_menu`, `outliner` (data model; omitted in audit), `eden_env→env` (omitted in audit); `eden_layout→editor/layout.rs` (omitted in audit); `eden_chrome` (T-661 shim, kept) → `editor/`. Renames imply self-guard edits (~120 sites); heaviest batch. Update `gate_t180.rs` `EDEN_CHROME`. Deep-relative includes in `zones_panel` (schema json, flatten.rs) gain one `../`.
- **T-934.6 — editor/state/ + editor/arsenal/ + mission_editor move**: `editor_session→state/session.rs`, `mission_commands→state/commands_hotkeys.rs` (name reserved for B2 layout — final naming pinned at .14), `mission_doc→state/doc_host.rs`, `mission_history→state/history.rs`, `mission_hydrate→state/hydrate.rs`, `mission_title_prefer→state/title_prefer.rs` (omitted in audit), `yrs_persist→state/persist.rs`, `editor_ops→state/operations.rs`; **`arsenal.rs`→`arsenal/mod.rs`** (preserves `crate::arsenal::class_r_scrub` path), `arsenal_rules, asset_catalog, arsenal_doll` → `arsenal/`; `mission_editor.rs`→`editor/mission_editor.rs`. Repoint all cross-guards targeting `editor_ops.rs` (8 origin files) and `mission_editor.rs` (eden_layout, eden_help/help_modal, zones_panel). Update `gate_t180.rs` `EDITOR_OPS`/`ORBAT_MGR` relatives, `gate_t439.rs`, `ai.rs:438`. **Phase A exit gate: `cargo xtask mk leptos-gates`.**

### Phase B — decomposition

- **T-934.7 — editor_ops split**: `state/operations.rs` becomes a `pub use` façade over `state/ops/{context, entity, compositions, attrs, transform, cargo}.rs`. Zero call-site churn. Acceptance list = all cross-guards into operations.rs re-pointed to the ops/ file now holding their pattern. Tree stays wasm32-gated.
- **T-934.8 — arsenal split**: `arsenal/mod.rs` keeps `ArsenalTab` + `class_r_scrub` (scrubber does NOT move); pure loadout core (~lines 224–1,250: `loadout_to_picks, picks_to_*, try_export/import, buffer_*, plan_*, commit_*`, receipts) → `arsenal/loadout.rs`; view fns (`cargo_panel, doll_view, attachments_panel, compat_panel, paper_doll`) → `arsenal/panels.rs`. Tests move adjacent; self-guards repointed.
- **T-934.9 — mission_editor test evacuation**: ~7.9k LOC of `#[cfg(test)]` blocks → `editor/mission_editor_tests/` via `#[cfg(test)] #[path]` mounts (precedent: `tbd_sat_pure`). Class S guards keep valid `include_str!` targets (`../mission_editor.rs` from the tests dir). Zero production change.
- **T-934.10 — pure helpers → editor/canvas/render_sync.rs**: lines ~2,317–3,338 (`connection_*, comment_*, hover_*, route_*, selectable_ids, crewed_slot_ids, map_render_*, filter_slot_soa_excluding, zone_centre, plain_paste_anchor`). Mechanical; tests follow.
- **T-934.11 — overlays → editor/canvas/overlays.rs**: `TransformWidgetOverlay, WidgetModeHint, SnapReadout, MapGridRefs, RulerOverlay, LosOverlay, AssetPickerOverlay, CommentEditorOverlay, ConnectionsPanelOverlay, ConflictDialog, BootProgressBar` + `AssetPickerState` + registries. `leptos-gates`.
- **T-934.12 — boot + RAF → editor/canvas/{boot, viewport}.rs** (audit §4 Phase 1 items 3–4): `BootProgress`/`BootEvent` + DEM/sat/chunk loaders → `boot.rs`; `start_raf`, `RenderDamage`, HUD builders, `device_size`, `register_*` → `viewport.rs`. Result: `mission_editor.rs` ≈ 4.1k (the `MissionEditorPage` body). `leptos-gates`.

### Phase B2 — gesture extraction (audit §4 Phase 2)

- **T-934.13 — EditorGestureContext + canvas/gestures.rs**: struct bundling the ~35 captured handles (NodeRefs, engine `Rc`, `DocHandle`, gesture `Rc`, `pan_px`, tool/los/snap/widget signals, `context_menu`, `asset_picker`, …); extract `onpointerdown/move/up`, `onwheel`, `ondblclick`, `oncontextmenu` closures as `gestures::attach_*(&ctx)`. Highest-risk child: full `leptos-gates` + manual smoke per gesture (pan, marquee, drag, rotate, wheel-zoom, context menu). Class S closure-body guards (`only_body(&ed, "let onpointerup =")`) re-pin to `gestures.rs`.
- **T-934.14 — hotkeys → state/commands.rs**: `commands::attach_editor_hotkeys(&ctx)` (Backspace/Delete exclusivity, Ctrl+Z/Y, G, `[`, `]`, 1/2/3…); resolve final naming vs `state/commands_hotkeys.rs` from .6. Final `MissionEditorPage` shell target ~450–800 LOC. `leptos-gates`.

### Phase C — backend

- **T-934.15 — handlers domain nesting** (all 22 mapped, none dropped; `lib.rs, app.rs, error.rs, state.rs, db.rs, config.rs, realtime.rs, auth/, middleware/, models/, contract/, services/` stay put):
  - `handlers/auth/{auth, oauth, dev, me}.rs`
  - `handlers/events/{events, factions}.rs`
  - `handlers/missions/{missions, approvals, registry}.rs`
  - `handlers/telemetry/{telemetry, servers, leaderboards, dashboard, deployments, field_tools}.rs`
  - `handlers/content/{cms, wiki, announcements, modpacks, mod_portal}.rs` (`mod` handler renamed — `mod.rs` reserved)
  - `handlers/admin/{admin, audit}.rs`
  `handlers/mod.rs` re-export façade keeps `app.rs` wiring minimal. Fix `gate_route_tags.rs` pin. `events.rs` (3,021 LOC) / `missions.rs` (2,911) splits are flagged follow-on candidates — not part of this program. Verify: `cargo xtask ci ci-local`.
- **T-934.16 — close-out**: full `ci ci-local` + `leptos-gates`; remap stale flat `owns` paths on queued tickets (e.g. T-926, T-930); `cargo xtask wave repack`; handoff note to Cursor for `docs/website/frontend/pages/*.md` link sync + CLAUDE.md §Status.

## 4. Migration table (all files)

### Frontend (73 flat + world_assets/9)

Rows marked `(omitted in audit)` were absent from audit table 3.5 and are placed here explicitly.

| Current (`apps/website/frontend/src/`) | Destination (`src/`) | Child |
|---|---|---|
| main.rs, router.rs, app_routes.rs | stay at root | — |
| auth.rs | core/auth.rs | .1 |
| client.rs | core/client.rs | .1 |
| dto.rs | core/dto.rs | .1 |
| sse.rs | core/sse.rs | .1 |
| datefmt.rs (omitted in audit) | core/datefmt.rs | .1 |
| toast.rs | core/toast.rs | .1 |
| ui.rs | core/ui.rs | .1 |
| url_guard.rs (omitted in audit) | core/url_guard.rs | .1 |
| split_pane.rs (omitted in audit) | core/split_pane.rs | .1 |
| layout.rs | shell/layout.rs | .1 |
| nav.rs | shell/nav_config.rs | .1 |
| dashboard.rs | pages/public/dashboard.rs | .2 |
| announcements.rs | pages/public/announcements.rs | .2 |
| server_intel.rs | pages/public/server_intel.rs | .2 |
| wiki.rs | pages/public/wiki.rs | .2 |
| vehicles.rs | pages/public/vehicles.rs | .2 |
| modpacks.rs | pages/public/modpacks.rs | .2 |
| mortar.rs | pages/public/mortar.rs | .2 |
| settings.rs | pages/public/settings.rs | .2 |
| deployments.rs | pages/public/deployments.rs | .2 |
| leaderboards.rs | pages/public/leaderboards.rs | .2 |
| events.rs | pages/operations/event_schedule.rs | .3 |
| event_hub.rs | pages/operations/event_hub.rs | .3 |
| orbat_selection.rs | pages/operations/orbat_selection.rs | .3 |
| orbat_manager.rs | pages/operations/orbat_manager.rs | .3 |
| faction_manager.rs | pages/operations/faction_manager.rs | .3 |
| event_manager.rs | pages/admin/event_manager.rs | .3 |
| approvals.rs | pages/admin/approvals.rs | .3 |
| server_control.rs | pages/admin/server_control.rs | .3 |
| personnel.rs | pages/admin/personnel.rs | .3 |
| content.rs | pages/admin/content.rs | .3 |
| audit.rs | pages/admin/audit.rs | .3 |
| missions.rs | editor/library/mission_library.rs | .4 |
| mission_overview.rs | editor/library/mission_overview.rs | .4 |
| create_mission_dialog.rs | editor/library/create_dialog.rs | .4 |
| select_tool.rs | editor/tools/select_tool.rs | .4 |
| ruler_tool.rs | editor/tools/ruler_tool.rs | .4 |
| los_tool.rs | editor/tools/los_tool.rs | .4 |
| place_helpers.rs | editor/tools/place_helpers.rs | .4 |
| world_assets/ (9 files) (omitted in audit) | editor/world_assets/ | .4 |
| world_layer_prefs.rs (omitted in audit) | editor/world_layer_prefs.rs | .4 |
| mission_size.rs (omitted in audit) | editor/mission_size.rs | .4 |
| eden_dock_left.rs | editor/panels/dock_left.rs | .5 |
| eden_dock_right.rs | editor/panels/dock_right.rs | .5 |
| eden_top_strip.rs | editor/panels/top_strip.rs | .5 |
| eden_toolbelt.rs | editor/panels/toolbelt.rs | .5 |
| eden_tree.rs | editor/panels/outliner_tree.rs | .5 |
| attributes.rs | editor/panels/attributes_modal.rs | .5 |
| validation_panel.rs | editor/panels/validation_panel.rs | .5 |
| eden_zones.rs | editor/panels/zones_panel.rs | .5 |
| eden_vehicles_panel.rs | editor/panels/vehicles_panel.rs | .5 |
| eden_settings.rs | editor/panels/settings_modal.rs | .5 |
| eden_help.rs | editor/panels/help_modal.rs | .5 |
| context_menu.rs | editor/panels/context_menu.rs | .5 |
| outliner.rs (omitted in audit) | editor/panels/outliner.rs | .5 |
| eden_env.rs (omitted in audit) | editor/panels/env.rs | .5 |
| eden_layout.rs (omitted in audit) | editor/layout.rs | .5 |
| eden_chrome.rs (omitted in audit; T-661 shim, kept) | editor/eden_chrome.rs | .5 |
| editor_session.rs | editor/state/session.rs | .6 |
| mission_commands.rs | editor/state/commands_hotkeys.rs | .6 |
| mission_doc.rs | editor/state/doc_host.rs | .6 |
| mission_history.rs | editor/state/history.rs | .6 |
| mission_hydrate.rs | editor/state/hydrate.rs | .6 |
| mission_title_prefer.rs (omitted in audit) | editor/state/title_prefer.rs | .6 |
| yrs_persist.rs | editor/state/persist.rs | .6 |
| editor_ops.rs | editor/state/operations.rs (façade after .7) | .6 |
| arsenal.rs | editor/arsenal/mod.rs | .6 |
| arsenal_rules.rs | editor/arsenal/arsenal_rules.rs | .6 |
| asset_catalog.rs | editor/arsenal/asset_catalog.rs | .6 |
| arsenal_doll.rs | editor/arsenal/arsenal_doll.rs | .6 |
| mission_editor.rs | editor/mission_editor.rs (then B/B2 splits) | .6 |

### Backend (`apps/website/api/src/handlers/`, 22 files)

All moves per Phase C child T-934.15 table above; everything else in `api/src/` stays put.

## 5. Cross-file `include_str!` guard inventory (repoint list)

Self-includes (~350 sites) need edits only where the file is *renamed* (events, missions, create_mission_dialog, all `eden_*` renames, attributes, arsenal→mod). Cross-file refs (must be repointed when geometry changes):

| Origin | Targets (count) |
|---|---|
| mission_editor.rs | editor_ops (15), mission_history (11), world_assets/* (10), select_tool (3), attributes (3), eden_toolbelt (2), eden_help (2), map-engine-core store.rs (2, depth +1 at .6), mission_hydrate (1), eden_tree (1) |
| attributes.rs | editor_ops (10), mission_history (1) |
| eden_dock_right.rs | editor_ops (9), eden_zones (1), fixtures (2) |
| eden_help.rs | mission_editor (3), eden_top_strip (2), orbat_manager (1), mission_history (1), faction_manager (1), eden_settings (1), context_menu (1), attributes (1) |
| eden_layout.rs | mission_editor (3), select_tool (1), eden_top_strip (1) |
| orbat_manager.rs | editor_ops (3), mission_history (1), attributes (1) |
| arsenal.rs | editor_ops (3), gap_analysis.md (depth), x.rs (literal test string — verify in place) |
| eden_zones.rs | editor_ops (2), mission_editor (1), mission.schema.json (depth), flatten.rs (depth) |
| eden_tree.rs | editor_ops (1), eden_dock_left (1) |
| eden_settings.rs | eden_zones (2) |
| mission_title_prefer.rs | mission_hydrate (2) |
| ui.rs | orbat_manager (1) |
| sse.rs | server_intel (1) |
| main.rs | sse (1) |
| content.rs | client (1) |
| mission_overview.rs | missions (1) |
| ruler_tool.rs / eden_dock_left.rs | world_assets/mod (1 each) |
| los_tool.rs / eden_toolbelt.rs | world_assets/dem_vectors (1 each) |
| missions.rs | ../Cargo.toml (1) |
| server_intel.rs / asset_catalog.rs / eden_dock_right.rs | ../tests/fixtures/* (4), registry.json depth (1) |

Regenerate before each child: `cd apps/website/frontend/src && grep -o 'include_str!("[^"]*")' -r . | sort | uniq -c`.

## 6. Verification matrix

| Children | Gate |
|---|---|
| .1–.5, .7–.10 | `cargo xtask mk ci-local-leptos` |
| .6 (Phase A exit), .11–.14 | + `cargo xtask mk leptos-gates`; .13 adds manual gesture smoke |
| .15, .16 | `cargo xtask ci ci-local` (+ `leptos-gates` at .16) |

## 7. Risks

1. include_str guards — per-batch inventory (§5); façades minimize churn; native tests fail loudly.
2. Gesture extraction (.13) — reactive capture in `'static` wasm closures; context-struct pattern per audit §4.2, sequenced last, browser-gated.
3. xtask/tools path pins — per-commit grep sweep; gates self-verify.
4. wasm32 cfg-gate fidelity — spec pins gated mod tree; both targets compiled by ci-local-leptos.
5. In-flight collision (operator-accepted) — Phase A as one focused burst; single wave repack + owns remap at close-out.
