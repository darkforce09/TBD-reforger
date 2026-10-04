# Mission Creator workspace

The `mission_creator_workspace` crate: the top of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the top-down 2D CAD workspace
in which mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) on the map. It
holds the editor page and its canvas mount, the docked chrome around the map (left and right docks,
top strip, toolbelt, context menu), the Editor Layers outliner, the inspectors, the full-screen
dialogs and the read-only review workspace that opens the editor on a submitted version.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/
├── Cargo.toml  the package: the four lower Mission Creator crates, the foundation and feature crates, the mission and editing crates, layout tier 12, any target
└── src/        the editor page, the review workspace, the docks, outliner, inspectors and dialogs, and the editor's top-level tests
```

## How it works

`MissionEditorPage` fills the lower crates' registered hooks (the draft-persist hook, the
right-click opener and the compile-findings publisher), creates the page signals and hands them to
the canvas mount, which boots the render engine through `mission_creator_engine_bridge` while
`mission_creator_session` restores the mission document, then loads the item
[registry](/documentation/glossary/n_to_z.md#registry) and raises the docks, toolbelt and overlays
around the map. Every panel reads signals and changes the document only through the hosted editing
commands. `ReviewWorkspacePage` mounts the same page in the state crate's read-only review mode.
The [source tree README](src/README.md) walks through each folder.

Everything that touches `web_sys` or a live engine handle compiles for `wasm32` only; an item only
browser code and the tests read carries `#[cfg(any(test, target_arch = "wasm32"))]`, so the native
library build holds the pure half and the native tests cover it.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_creator_workspace   # the panels' pure models and the editor's source pins
```

## Configuration

None.

## Public surface

- `MissionEditorPage` and `ReviewWorkspacePage` (wasm32), the two route components the app's route
  table mounts; `prelude` re-exports them.
- The `mission_editor`, `review_workspace` and `ui` modules and their documented items.

## Boundaries

- Depends on: `mission_creator_state`, `mission_creator_engine_bridge`, `mission_creator_session`,
  `mission_creator_arsenal`, `mission_review_record`, the foundation crates, the mission and
  editing crates, and the map crates in the browser build.
- Used by: the app (`apps/frontend/src/app_routes.rs`).
- Rules: the top of the Mission Creator crate order; no lower crate depends on it
  (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) —
  the roadmap, specifications and decisions.
- [Workspace crates](/crates/frontend/workspaces/README.md) — the Mission Creator crate order.
