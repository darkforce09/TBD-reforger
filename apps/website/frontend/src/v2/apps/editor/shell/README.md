# Browser session

Everything the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) keeps per browser
tab rather than in the [mission](/documentation_v2/glossary/g_to_m.md#mission) document: the IndexedDB
draft writer and its save status, the server hydrate and the snapshots that undo it, the cross-tab
writer role, the read-only review mode, the warm-session marker, the chrome layout, the world-layer
preferences, the payload-size readout, and the browser transport behind Save, Export, the merge and
the clipboard commands.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/shell/
├── document_commands/    the wasm-only transport files of the document commands
├── document_commands.rs  the command context, the mission-row cells, the `__editorCommands` bridge
├── eden_chrome.rs        re-exports: chrome components, inset constants, zone predicates
├── hydrate/              the server fetch, the draft-versus-server verdict, the snapshot pair
├── hydrate.rs            the hydrate's module root and the `__missionBackup` bridge; wasm-only
├── layout.rs             chrome dimensions, live insets, collapse state, pane centre, class recipes
├── mission_size.rs       the compiled payload-size estimate and the byte formatter
├── mod.rs                the module tree
├── persist/              the account-scoped IndexedDB records and the debounced write
├── persist.rs            the draft writer's root: database coordinates, merge, cross-tab sync
├── review_mode.rs        the reviewed version of the review workspace and the write predicate
├── save_status.rs        the save status, its self-mounted chip, one toast per failed episode
├── session.rs            the account-scoped warm-editor marker in `sessionStorage`
├── tab_lock/             the browser transport of the writer role: Web Lock, channel, stamps
├── tab_lock.rs           the writer role, the save decision, the election and the read-only banner
├── tests/                unit tests for the session's modules, source pins included
├── title_prefer.rs       mounts the tests of the title preference and the mission-row metadata wire
└── world_layer_prefs.rs  the world-layer and basemap preferences in `localStorage`, with migration
```

## How it works

```text
boot (mission_editor/canvas_mount/)
├── review_mode holds this mission ──> restore the reviewed version; arm nothing
└── otherwise ──> persist: draft from IndexedDB ──> hydrate: GET /api/v1/missions/{id}
                  ──> Empty / MatchesServer / Diverged (conflict dialog)
                  ──> arm the draft writer, session marker, flush on hide, tab sync
edit ──> undo driver tail ──> persist::schedule_edit_persist ──> run_save
         ├── tab_lock: writer, read-only (defer) or merge first
         └── save_status: saving, saved or failed (chip, toast)
Save Version / Export / merge ──> document_commands ──> API, download, toasts
```

Review mode is opened by the review workspace page before it mounts the editor, on the version an
[artifact](/documentation_v2/glossary/a_to_f.md#artifact) compiled from, and closed when that page goes
away. While it is open every write path consults `review_mode::writes_mission`: the boot restores
the reviewed version instead of the draft and the server's current version, the draft writer is
never armed, the writer role reads as read-only without an election, the unload prompt stays off,
Save Version refuses, and neither mission-row mirror patches the row; Export Compiled compiles over
the row fields the artifact recorded.

`layout.rs` owns the chrome's numbers: the 48 px top strip, the 240 px docks with their 24 px
collapsed stub, and the 96 px toolbelt band. The live accessors (`dock_left_px`, `dock_right_px`,
`strip_top_px`, `toolbelt_band_px`) follow the collapse and hidden flags the canvas mount sets, and
the pointer gestures and the select tool read them to tell the map from the chrome; the collapse and
hidden flags last as long as the page. The world-layer and basemap preferences persist in
`localStorage` under `tbd-mc-editor-prefs`. The warm-session marker records, per account, that this
tab finished booting a mission (its id, [slot](/documentation_v2/glossary/n_to_z.md#slot) count and time);
only the `__missionPersist.warm()` probe reads it back. The in-memory state here (the save status,
the tab role, the snapshot cache, the review cell) dies with the tab, while the drafts, snapshots,
preferences and marker it stores outlive it.

## Public surface

- `review_mode` (`ReviewedVersion`, `open`, `close`, `reviewed_for`, `writes_mission`): the review
  workspace page, the boot, the undo driver's unload guard and the mission-row mirrors.
- `hydrate::purge_local_documents`: the auth store's sign-out.
- `mission_size::{estimate_compiled_bytes, format_bytes}`: the page effects, the toolbelt, the top
  strip and the mission library's upload panel.
- `layout`: the live insets for the input layer, the dock mount classes for the page, the collapse
  setters and pane-centre hold for the canvas mount, and the `HOVER_FILL`, `TOGGLED_PLATE` and
  `DISABLED_GLYPH` class recipes for the editor's surfaces and the core search box, select and
  slider.
- `eden_chrome`: `DockLeft`, `DockRight`, `TopCommandStrip`, `MissionSettingsDialog` and
  `OrbatManagerDialog` for the page, and the zone predicates for the zone draw. It also re-exports
  `BottomToolbelt`, which nothing mounts (the page mounts `ModeToolbar` and `StatusBar` from the
  toolbelt directly), and the four inset constants, which every reader takes from `layout` instead.
- `tab_lock::TabLockBanner` for the page; `world_layer_prefs` for the world-assets host and the
  preferences dialog; `document_commands` for the top strip, the
  [Arsenal](/documentation_v2/glossary/a_to_f.md#arsenal) tab, the mission-row mirrors and the boot;
  `persist` and `session` for the boot.

## Boundaries

- Depends on: `website_map_engine::editing::persist` (record keys, stored blobs, merge policy,
  local-versus-server verdict, server adoption, snapshot slots) and `editing::commands`,
  `data::scenario` for the compile, `data::store` (`MissionDocCore`,
  `operations::slot_ids::duplicate_slot_ids`), `streaming::bridge` for the boot progress and the
  preference types; the `DocHandle` and undo driver of
  `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`; `crate::v2::core` (the
  [API](/documentation_v2/glossary/a_to_f.md#api) client and DTOs, the auth store, the toasts, the
  clipboard helper); `idb`, `gloo_net`, `web_sys` and `js_sys`; over HTTP,
  `GET /api/v1/missions/{id}`, `POST /api/v1/missions/{id}/versions` and
  `GET /api/v1/missions?scope=mine`.
- Used by:
  - in `apps/website/frontend/src/v2/apps/editor/`: the editor page `mission_editor.rs` and its
    canvas mount, the undo driver and world-assets host in `bridge/`, the pointer gestures and
    select tool in `input/`, the docks, inspectors and dialogs in `ui/`, and the Arsenal tab;
  - `apps/website/frontend/src/v2/pages/mission_hub/review_workspace/page.rs` (review mode),
    `apps/website/frontend/src/v2/core/auth/store.rs` (the sign-out purge), the upload panel in
    `apps/website/frontend/src/v2/pages/mission_hub/library/` (`format_bytes`), and the search box,
    select and slider in `apps/website/frontend/src/v2/core/ui/` (the class recipes);
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, through
    `window.__missionPersist` and `window.__editorCommands`;
  - `cargo xtask verify editor-orbat-coherency`, which scans `eden_chrome.rs`.
- Rules:
  - while review mode is open nothing is written: no draft, no marker, no election, no version, no
    row change (`the_review_boot_restores_the_reviewed_version_and_arms_nothing` and its neighbours
    in `tests/review_mode/read_only_review.rs`);
  - every inset has one definition in `layout.rs`, and the readers use the live accessors
    (`both_readers_reference_the_single_band_const` in `tests/layout/band_readers.rs`);
  - a module that touches `web_sys` or a live document handle is `#[cfg(target_arch = "wasm32")]`,
    and so is its `pub mod` line, so the save policy, the election, the size arithmetic and the
    layout are tested natively;
  - what a command decides belongs to `website_map_engine::editing::commands`, and nothing here
    draws a frame.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) — the local draft, hydrate, conflict dialog and tab lock.
- [Mission Creator feature inventory: map basemap and world objects](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_basemap_and_world_objects.md) — the per-user basemap and world-layer preferences.
- [Mission Creator decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) — the
  load-conflict, autosave, undo and editor-session decisions.
- [Mission Creator feature inventory: shell route and layout](/documentation_v2/website/frontend/apps/editor/feature_inventory/shell_route_and_layout.md) — the review mode and the chrome layout.
- [Mission Creator feature inventory: performance at scale](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md) — the load, save and warm-session behaviour at scale.
