# Frontend workspace crates

The workspaces layer of the frontend crates: the standalone full-screen applications the app hosts
beside its pages, each mounting its own canvas and driving the map and graphics engines directly —
the Mission Creator, split into its crate order, and the URL-only debug benches.

## Contents

```text
crates/frontend/workspaces/
├── debug_benches/          `debug_benches`: the building viewer, world line of sight, equipment data viewer and ballistics agreement benches
├── mission_creator_arsenal/  `mission_creator_arsenal`: the Mission Creator's Arsenal: the loadout editor tab, its loadout core and the paper doll host
├── mission_creator_engine_bridge/  `mission_creator_engine_bridge`: the Mission Creator's engine bridge and input layer
├── mission_creator_session/  `mission_creator_session`: the Mission Creator's browser session: drafts, hydrate, writer role, Save and Export
├── mission_creator_state/  `mission_creator_state`: the Mission Creator's lowest crate, its pure editor state
└── mission_creator_workspace/  `mission_creator_workspace`: the Mission Creator's top crate: the editor page, docks, outliner, inspectors, dialogs and review workspace
```

## How it works

A workspace crate owns its whole surface: its canvas, its signals and its engine handle. The
app's route table (`apps/frontend/src/app_routes.rs`) mounts each workspace's route components,
which exist on `wasm32`; the pure halves under them compile on every target, so their tests run
natively.

## Public surface

- `debug_benches`: `BuildingViewerPage`, `WorldLosPage`, `DataViewerPage` and
  `BallisticsAgreementPage`; see its README.
- `mission_creator_arsenal`: `ArsenalTab` and the loadout core; see its README.
- `mission_creator_engine_bridge`: see its README.
- `mission_creator_session`: see its README.
- `mission_creator_state`: see its README.
- `mission_creator_workspace`: `MissionEditorPage` and `ReviewWorkspacePage`; see its README.

## Boundaries

- Depends on: the foundation crates under `crates/frontend/foundation/`, the feature crates under
  `crates/frontend/features/`, and the engine crates under `crates/`.
- Used by: the app (`apps/frontend`), whose route table mounts the workspaces.
- Rules: a workspace crate never depends on a page crate or the app; the Mission Creator's crates
  follow their own order (`mission_creator_state` < `mission_creator_engine_bridge` <
  `mission_creator_session` < `mission_creator_arsenal` < `mission_creator_workspace`, each
  depending only on crates before it), and the debug benches depend on no Mission Creator crate
  and none depends on them (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Frontend crates](/crates/frontend/README.md) — the layer order every frontend crate follows.
- [Frontend source root](/apps/frontend/src/README.md) — the app's entry point, route table and
  frame, and the layer order of the frontend crates.
