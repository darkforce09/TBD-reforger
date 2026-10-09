# Mission editing crates

The crates of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s editing
layer over the [mission](/documentation/glossary/g_to_m.md#mission) document: the editing session
that hosts the live document, and the decidable half of local drafts. They hold no browser, UI
framework or GPU code; the Mission Creator injects clocks, prompts, storage and hooks as closures.

## Contents

```text
crates/mission_editing/
├── map_editing_tools/        the headless map tools: selection, ruler, line of sight, viewshed job scheduler
├── mission_editing_commands/  the hosted document commands and the pure export, report and selection texts
├── mission_editing_session/  the hosted document, undo, grouping, routing, selection, picks and overlay lanes
└── mission_persistence/      local draft decisions: record keys, blob verdicts, merge, reconciliation, snapshots
```

## How it works

`mission_editing_session` sits on the mission crates (`mission_document`, `mission_crdt`,
`mission_validation`) and the engine crates the picks and lanes read (`camera_math`,
`spatial_indexes`, `unit_symbology`); `mission_persistence` sits on the session's document handle
and the payload compiler; `mission_editing_commands` sits on the session's host and post-change
tail and on `mission_operations`; `map_editing_tools` sits on the session's picks and the
line-of-sight, elevation, camera and point-index crates. The Mission Creator links each crate
directly.

## Boundaries

- Depends on: the foundation, mission and engine CPU crates; never a browser crate.
- Used by: the Mission Creator in `crates/frontend/shell/frontend_application/`, which links each crate directly.
- Rules: every edge points to a lower tier along the category matrix
  (`cargo xtask verify crate-tiers`); every crate keeps the library anatomy
  (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Library crates](/crates/README.md) — the categories and the crate laws.
- [Mission crates](/crates/mission/README.md) — the document and operations these crates edit.
