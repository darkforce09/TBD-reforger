# Mission Creator

The [Mission Creator](/documentation_v2/glossary.md#mission-creator): the 2D/3D CAD workspace in
which mission makers build a [mission](/documentation_v2/glossary.md#mission) on the map. This
folder holds the editor page, the chrome docked around the map, the canvas mount and its overlays,
the interactive map tools, the [arsenal](/documentation_v2/glossary.md#arsenal) loadout editor and
the browser session they all run in.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/
├── arsenal/           the Arsenal tab and loadout domain; the asset catalog trees the palettes read
├── bridge/            the engine seam: boot, document host and undo, host state, viewport, overlays
├── input/             pointer and keyboard events turned into map-engine commands; the map tools
├── mission_editor/    the page's parts: canvas mount, registries, placement, transform, toolbar
├── mission_editor.rs  `MissionEditorPage`, which mounts the canvas and raises the chrome around it
├── mod.rs             the module tree
├── shell/             the browser session: drafts, hydrate, tab lock, review mode, preferences
├── tests/             unit tests for the page and its source pins, mounted from `mission_editor.rs`
└── ui/                docks, top strip, toolbelt, outliner, inspectors, Arsenal panels and dialogs
```

## How it works

`MissionEditorPage` creates the page signals and hands them to the canvas mount in
`mission_editor/`, which in the browser boots the render engine through `bridge/` while it
restores the mission document from the server and the local draft (`shell/`), then loads the item
[registry](/documentation_v2/glossary.md#registry) and the compatibility feed and raises the docks,
toolbelt and overlays around the map. `mission_editor/` holds no `mod.rs`: `mission_editor.rs`
declares each of its files by path.

Pointer and keyboard input (`input/`) and every panel under `ui/` change the document only through
the map engine's hosted editing commands, which `bridge/` hosts together with the document handle,
the undo history, the selection and the armed placement; the panels read signals and never write
the document themselves. A module that touches `web_sys` or a live engine handle compiles for
`wasm32` only, and its `pub mod` line carries the same gate, so the native test build still
compiles the pure half of the workspace. When the review workspace opens an
[artifact](/documentation_v2/glossary.md#artifact)'s version, the review mode in `shell/` holds it
and every write path refuses.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage`, from the mission hub's review workspace, which mounts `MissionEditorPage` in review mode | `mission_maker` | full-bleed, chromeless |

## Public surface

- `mission_editor::MissionEditorPage`: the route component, mounted by the route table and by the
  review workspace page.
- `shell::review_mode` (`ReviewedVersion`, `open`, `close`): the read-only review mode the review
  workspace opens before it mounts the editor.
- `shell::hydrate::purge_local_documents`: drops an account's local drafts; the auth store calls it
  when the account signs out.
- `shell::mission_size::format_bytes`: the payload-size formatter the mission library's upload
  panel reuses.
- `shell::layout::{HOVER_FILL, DISABLED_GLYPH}`: the chrome's hover and disabled classes, reused by
  the core search box, select and slider.

## Boundaries

- Depends on: `crate::v2::core` (the [API](/documentation_v2/glossary.md#api) client and DTOs, the
  auth store, the UI primitives and utilities, the test support), `website_map_engine` (its
  `data`, `editing`, `streaming`, `overlay`, `frame`, `camera`, `spatial`, `world` and `doll`
  modules) and `web_sys` in the browser build.
- Used by:
  - `apps/website/frontend/src/app_routes.rs`, the route table;
  - `apps/website/frontend/src/v2/pages/mission_hub/review_workspace/page.rs`, for the page and the
    review mode, and its tests, which also read `shell::tab_lock::may_write`;
  - in `apps/website/frontend/src/v2/pages/mission_hub/library/`, `dossier_upload.rs` and
    `dossier_upload_panel.rs`, for `format_bytes`;
  - `apps/website/frontend/src/v2/core/auth/store.rs`, for `purge_local_documents`;
  - the core search box, select and slider in `apps/website/frontend/src/v2/core/ui/`, for the
    layout classes;
  - source pins that read this folder's files:
    `apps/website/frontend/src/v2/core/test_support/editor_operations.rs`,
    `apps/website/frontend/src/v2/core/ui/tests/ui.rs` and two map-engine tests under
    `apps/website/map-engine/src/`;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, which drive the
    `/missions/:id/edit` route, and `cargo xtask verify editor-orbat-coherency`, which scans named
    files under `arsenal/`, `bridge/`, `shell/` and `ui/modals/`.
- Rules: a document mutation goes through `website_map_engine::editing`, never straight out of a
  panel; a module that touches `web_sys` or a live engine handle is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line, which the native
  `cargo test -p website-frontend` build holds; no sibling workspace reaches in, and production
  code in pages and core imports only the items under Public surface, which no gate checks. The
  coherency gate names its files by path, so moving one of them breaks
  `cargo xtask verify editor-orbat-coherency`.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  entry point to the roadmap, the specifications and the decisions.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md) — every feature by area, with its status in the code.
- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout, the gestures, the shortcuts and the load and save flow.
- [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  — what the editor ships by area, and the open and deferred work.
- [Editor gates runbook](/documentation_v2/runbooks/editor_gates.md) — running the headless editor
  gates.
