# Mission Creator workspace source

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the top-down 2D CAD
workspace in which mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) on the
map. This folder holds the editor page, the chrome docked around the map, the canvas mount and its overlays,
the interactive map tools and the read-only review workspace that opens the editor on a submitted
version; the browser session they all run in is the `mission_creator_session` crate, and the
[arsenal](/documentation/glossary/a_to_f.md#arsenal) loadout editor the Attributes dialog mounts is
the `mission_creator_arsenal` crate.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/
├── error.rs           the crate's error: why an inspector or dialog edit was refused
├── lib.rs             the crate root: the module tree and the two route components
├── mission_editor/    the page's parts: canvas mount, registries, toolbar dispatch, page effects
├── mission_editor.rs  `MissionEditorPage`, which mounts the canvas and raises the chrome around it
├── prelude.rs         the two route components, for a glob import
├── review_workspace/  the Mission Creator, read-only, on the version an artifact compiled from
├── tests/             unit tests for the page and its source pins, and the review mode's write pins
└── ui/                docks, top strip, toolbelt, outliner, inspectors and dialogs
```

## How it works

`MissionEditorPage` creates the page signals and hands them to the canvas mount in
`mission_editor/`, which in the browser boots the render engine through the engine bridge while it
restores the mission document from the server and the local draft (`mission_creator_session`), then loads the item
[registry](/documentation/glossary/n_to_z.md#registry) and the compatibility feed and raises the docks,
toolbelt and overlays around the map. `mission_editor/` holds no `mod.rs`: `mission_editor.rs`
declares each of its files by path.

The folders form layers above four crates: `mission_creator_state`
(`crates/frontend/workspaces/mission_creator_state/`), the pure state every layer reads,
`mission_creator_engine_bridge` (`crates/frontend/workspaces/mission_creator_engine_bridge/`), the
engine bridge and the input layer, and `mission_creator_session`
(`crates/frontend/workspaces/mission_creator_session/`), the browser session above the bridge, and
`mission_creator_arsenal` (`crates/frontend/workspaces/mission_creator_arsenal/`), the Arsenal
above the session; `ui/`, `mission_editor/`, `review_workspace/` and `tests/` sit at the top. A lower layer reaches an upper one only through a
registered cell the upper layer fills at mount: the recently-placed recorder of the state crate, the
draft-persist hook in the bridge's `document_host`, the right-click opener of its input layer and the
compile-findings publisher of the session crate. `MissionEditorPage` fills the last three first thing,
before the canvas mount and the top strip exist, and a remount's registration replaces the
earlier one.

Pointer and keyboard input (the engine bridge crate's `input`) and every panel under `ui/` change
the document only through the map engine's hosted editing commands, which the bridge hosts together
with the document handle,
the undo history, the selection and the armed placement; the panels read signals and never write
the document themselves. A module that touches `web_sys` or a live engine handle compiles for
`wasm32` only, and its `pub mod` line carries the same gate, so the native test build still
compiles the pure half of the workspace. When the review workspace in
`review_workspace/` opens an [artifact](/documentation/glossary/a_to_f.md#artifact)'s version, the
review mode of the state crate holds it and every write path refuses. The review workspace is the
Mission Creator itself in its read-only review mode, not a copy of it.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/edit` | `MissionEditorPage` | `mission_maker` | full-bleed, chromeless |
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` in `review_workspace/`, which mounts `MissionEditorPage` in review mode | `mission_maker` | full-bleed, chromeless |

## Public surface

- `mission_editor::MissionEditorPage` and `review_workspace::ReviewWorkspacePage` (re-exported at
  the crate root and in `prelude`): the route components the route table mounts; the review
  workspace page mounts the editor page too.
- `error::Error` and `error::Result`: the refusal the inspectors' row edits and the ORBAT manager's
  update of a faction template from a side return.

## Boundaries

- Depends on: the `mission_creator_state` crate (the pure state vocabulary), the
  `mission_creator_engine_bridge` crate (the engine bridge and input), the
  `mission_creator_session` crate (drafts, hydrate, writer role, Save and Export), the
  `mission_creator_arsenal` crate (the Arsenal tab), the foundation crates (the [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the
  auth store, the UI primitives and utilities, the test support), the `mission_review_record` crate (the review
  workspace banner's review wording), the editing crates of `crates/mission_editing/`
  (`mission_editing_session`, `mission_editing_commands`, `mission_persistence`,
  `map_editing_tools`), the map crates (`map_streaming_model`, and `map_renderer` and `map_streaming_host` in the browser
  build) and `web_sys` in the browser build.
- Used by:
  - `apps/frontend/src/app_routes.rs`, the route table;
  - source pins that read this folder's files:
    a mission document test in `crates/mission/mission_document/src/rows/tests/cases_1.rs`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, which drive the
    `/missions/:id/edit` route, and `cargo xtask verify editor-orbat-coherency`, which scans named
    files under `ui/modals/`, the page `mission_editor.rs`, the engine bridge crate and the Arsenal
    crate.
- Rules: a document mutation goes through the hosted commands of `mission_editing_commands`,
  never straight out of a panel; a module that touches `web_sys` or a live engine handle is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line, which the native
  `cargo test -p mission_creator_workspace` build holds; no other frontend crate depends on this
  crate, and only the application root (`app_routes.rs`) imports the items under Public surface,
  which `cargo xtask verify frontend-layering` holds at zero edges. The coherency gate names its files by path, so moving one of them breaks
  `cargo xtask verify editor-orbat-coherency`.

## Related documentation

- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  entry point to the roadmap, the specifications and the decisions.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md) — every feature by area, with its status in the code.
- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the layout, the gestures, the shortcuts and the load and save flow.
- [Mission Creator roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md)
  — what the editor ships by area, and the open and deferred work.
- [Editor gates runbook](/documentation/runbooks/editor_gates.md) — running the headless editor
  gates.
- [Mission Creator feature inventory: shell route and layout](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/shell_route_and_layout.md) — the chromeless route, the chrome layout and the review workspace.
