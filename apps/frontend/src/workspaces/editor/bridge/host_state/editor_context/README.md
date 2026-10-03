# Editor context

The context the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) installs once at
load: the document, render-engine and selection handles every panel reaches the open
[mission](/documentation/glossary/g_to_m.md#mission) through, the Leptos signals that mirror the
document into the docks, and the side signals that open the asset picker, the comment editor and the
Connections panel.

## Contents

```text
apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/
├── attributes_modal.rs  opens and closes the Attributes dialog and adjusts the selection to match
├── dock_mirrors.rs      pushes the document into the dock signals; the Connections panel signals
├── document_fields.rs   the environment block, the title and the slot roster, read and written
├── installation.rs      `install`, the context and armed-value types, picker and comment signals
└── mod.rs               the module tree; re-exports; `EDITOR_CONTEXT` and the other thread-locals
```

## How it works

The canvas mount calls `install` once, after the document is seeded, and registers the page's asset
picker, comment editor, Connections panel and connection selection signals beside it. The
`EditorContext` it installs holds the `DocHandle`, the `EngineHandle` and the `SelectionHandle`, the
active layer, active side and Objects-mode signals, the dock mirrors (outliner nodes,
[ORBAT](/documentation/glossary/n_to_z.md#orbat) nodes, selected ids), the Attributes dialog's open id
and tab, the document tick, the armed `Pending` of a placement and the id minter. The handles are
`!Send` `Rc`s, so the context lives in the `EDITOR_CONTEXT` thread-local rather than down the
component tree, and a remount installs a new one over the old.

`MissionDocCore` has no change subscription, so the undo driver's tail calls `refresh_docks` after
every mutation: it rebuilds the outliner tree (layers, [slots](/documentation/glossary/n_to_z.md#slot),
comments) and the ORBAT tree (factions, squads, slots) from the document's projections, mirrors the
selection, clears a connection selection that an entity selection or a delete has made stale, and
bumps the document tick the panels re-read on; `bump_doc_tick` bumps it alone, for a draw that has
not written yet. The field accessors read the title and the `meta.environment` block and answer the
empty value when no document is installed; `set_title` and `update_environment` write and run
`after_local_edit`. `open_attributes` and `open_arsenal` (tab 3) keep a multi-selection that already
holds the target, so the dialog edits every selected entity, and otherwise collapse the selection to
the target. `seed_new_mission_template` writes the comments a fresh mission starts with, under the
init origin, before the draft restore and the server hydrate replace the document.

## Boundaries

- Depends on: `mission_operations` (`projections` for the layer, faction, squad and slot rows,
  `entity` for comment rows and connection ids, `environment::read_env`);
  `map_editing_tools::selection`; `map_engine` (`frame::EngineHandle`); the undo driver and `DocHandle` in
  `apps/frontend/src/workspaces/editor/bridge/document_host/`; the outliner's node builders;
  the asset catalog's `PlacePayload`; `MissionEnv` from `crate::foundation::transport::dto`.
- Used by:
  - the canvas mount, its document setup and the page effects in
    `apps/frontend/src/workspaces/editor/mission_editor/`, which install and read it;
  - in `apps/frontend/src/workspaces/editor/bridge/`: the armed placement, the entity
    selection, the undo driver, the overlays, the tactical-graphics authoring and the world-assets
    host;
  - the pointer gestures and the double-click in
    `apps/frontend/src/workspaces/editor/input/pointer_gestures/`;
  - the docks, inspectors, dialogs and outliner rows under
    `apps/frontend/src/workspaces/editor/ui/`, and the loadout attachments in
    `apps/frontend/src/workspaces/editor/arsenal/loadout/`;
  - the source pins that read these files through `CONTEXT` in
    `apps/frontend/src/foundation/test_support/editor_operations.rs`.
- Rules: every entry point opens exactly one borrow of the context, and a document borrow drops
  before the post-edit tail opens its own; the module is `#![cfg(target_arch = "wasm32")]`, and so
  is its `pub mod` line; an unregistered side signal makes open and close a no-op; every file here
  is on the place path that `cargo xtask verify editor-orbat-coherency` scans, which fails when a
  listed file is missing.

## Related documentation

- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md)
  — the outliner, the ORBAT tree and the Attributes dialog these signals feed.
