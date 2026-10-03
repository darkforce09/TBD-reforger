# Mission Creator

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the top-down 2D CAD
workspace in which mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) on the
map. This folder holds the editor page, the chrome docked around the map, the canvas mount and its overlays,
the interactive map tools, the [arsenal](/documentation/glossary/a_to_f.md#arsenal) loadout editor,
the browser session they all run in, and the read-only review workspace that opens the editor on a
submitted version.

## Contents

```text
apps/frontend/src/workspaces/editor/
├── arsenal/           the Arsenal tab and loadout domain; the asset catalog trees the palettes read
├── bridge/            the engine seam: boot, document host and undo, host state, viewport, overlays
├── input/             pointer and keyboard events turned into map-engine commands; the map tools
├── mission_editor/    the page's parts: canvas mount, registries, placement, transform, toolbar
├── mission_editor.rs  `MissionEditorPage`, which mounts the canvas and raises the chrome around it
├── mod.rs             the module tree
├── review_workspace/  the Mission Creator, read-only, on the version an artifact compiled from
├── session/           the browser session: drafts, hydrate, tab lock, review mode, preferences
├── test_support.rs    test-only: the production half of a source file, for the source pins
├── tests/             unit tests for the page and its source pins, and for `test_support.rs`
└── ui/                docks, top strip, toolbelt, outliner, inspectors, Arsenal panels and dialogs
```

## How it works

`MissionEditorPage` creates the page signals and hands them to the canvas mount in
`mission_editor/`, which in the browser boots the render engine through `bridge/` while it
restores the mission document from the server and the local draft (`session/`), then loads the item
[registry](/documentation/glossary/n_to_z.md#registry) and the compatibility feed and raises the docks,
toolbelt and overlays around the map. `mission_editor/` holds no `mod.rs`: `mission_editor.rs`
declares each of its files by path.

Pointer and keyboard input (`input/`) and every panel under `ui/` change the document only through
the map engine's hosted editing commands, which `bridge/` hosts together with the document handle,
the undo history, the selection and the armed placement; the panels read signals and never write
the document themselves. A module that touches `web_sys` or a live engine handle compiles for
`wasm32` only, and its `pub mod` line carries the same gate, so the native test build still
compiles the pure half of the workspace. When the review workspace in
`review_workspace/` opens an [artifact](/documentation/glossary/a_to_f.md#artifact)'s version, the
review mode in `session/` holds it and every write path refuses. The review workspace is the
Mission Creator itself in its read-only review mode, not a copy of it.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` in `review_workspace/`, which mounts `MissionEditorPage` in review mode | `mission_maker` | full-bleed, chromeless |

## Public surface

- `mission_editor::MissionEditorPage` and `review_workspace::ReviewWorkspacePage`: the route
  components the route table mounts; the review workspace page mounts the editor page too.
- `session::hydrate::purge_local_documents`: drops an account's local drafts; `main.rs` registers
  it as the auth store's sign-out hook, which runs when the account signs out.

## Boundaries

- Depends on: `crate::foundation` (the [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the
  auth store, the UI primitives and utilities, the test support), `crate::features` (the review
  workspace banner's review wording), the editing crates of `crates/mission_editing/`
  (`mission_editing_session`, `mission_editing_commands`, `mission_persistence`,
  `map_editing_tools`), the map crates (`map_renderer`, `gpu_frame`, `map_streaming_host`,
  `map_streaming_model`, `map_asset_loading`, `map_render_diagnostics`, `paper_doll_renderer`) and `web_sys` in the browser build.
- Used by:
  - `apps/frontend/src/app_routes.rs`, the route table;
  - `apps/frontend/src/main.rs`, which registers `purge_local_documents` as a sign-out hook;
  - source pins that read this folder's files:
    `apps/frontend/src/foundation/test_support/editor_operations.rs`,
    `apps/frontend/src/foundation/ui/tests/ui.rs` and a mission document test in
    `crates/mission/mission_document/src/rows/tests/cases_1.rs`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, which drive the
    `/missions/:id/edit` route, and `cargo xtask verify editor-orbat-coherency`, which scans named
    files under `arsenal/`, `bridge/`, `session/` and `ui/modals/`.
- Rules: a document mutation goes through the hosted commands of `mission_editing_commands`,
  never straight out of a panel; a module that touches `web_sys` or a live engine handle is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line, which the native
  `cargo test -p frontend` build holds; no sibling workspace, page, feature or foundation
  module imports from this folder, and only the application root (`main.rs`, `app_routes.rs`)
  imports the items under Public surface, which `cargo xtask verify frontend-layering` holds at
  zero edges. The coherency gate names its files by path, so moving one of them breaks
  `cargo xtask verify editor-orbat-coherency`.

## Related documentation

- [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md) — the
  entry point to the roadmap, the specifications and the decisions.
- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md) — every feature by area, with its status in the code.
- [Mission Creator UX specification](/documentation/apps/frontend/workspaces/editor/ux_spec.md) —
  the layout, the gestures, the shortcuts and the load and save flow.
- [Mission Creator roadmap](/documentation/apps/frontend/workspaces/editor/mission_creator_roadmap.md)
  — what the editor ships by area, and the open and deferred work.
- [Editor gates runbook](/documentation/runbooks/editor_gates.md) — running the headless editor
  gates.
- [Mission Creator feature inventory: shell route and layout](/documentation/apps/frontend/workspaces/editor/feature_inventory/shell_route_and_layout.md) — the chromeless route, the chrome layout and the review workspace.
