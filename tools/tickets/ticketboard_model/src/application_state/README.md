# Application state

The state of the ticketboard application: what `TicketboardApp` in `apps/ticketboard` owns and
changes, without painting it.

## Contents

```text
tools/tickets/ticketboard_model/src/application_state/
├── background_loading.rs  `LoadBundle` and `spawn_load`, the combined load on a worker thread
├── events.rs              the `Tab` list and `Action`, with a conversion from each feature's events
├── mod.rs                 the module tree
├── preferences.rs         the eframe storage keys and the saved viewer width, clamped on load
├── tests/                 unit tests for the window layout and workspace reloads
├── window_layout.rs       which right-hand columns fit: detail, document viewer or both
├── workspace_reload.rs    `WorkspaceState::reload`, keeping selections, filters and sorts
└── workspace_state.rs     the `State` machine and `WorkspaceState`, the loaded board
```

## Boundaries

- Depends on: every feature module of the crate.
- Used by: `apps/ticketboard/src/application/`.
- Rules: no feature imports this module
  (`model_dependency_boundaries_and_external_test_placement_are_enforced` in
  `tools/tickets/ticketboard_model/src/tests/architecture_rules.rs`).
