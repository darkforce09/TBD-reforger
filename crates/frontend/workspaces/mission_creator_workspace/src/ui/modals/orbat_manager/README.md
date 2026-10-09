# ORBAT Manager parts

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[ORBAT](/documentation/glossary/n_to_z.md#orbat) Manager: a near-full-screen dialog over the live ORBAT
of the open [mission](/documentation/glossary/g_to_m.md#mission), its sides, squads,
[slots](/documentation/glossary/n_to_z.md#slot) and squad vehicles, with a slot inspector beside the
tree and the faction library templates a side loads from or saves to. The parent module,
`crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/orbat_manager.rs`, declares these modules,
re-exports `OrbatManagerDialog` and the template helpers, and holds the tree's row constants.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/orbat_manager/
├── dialog.rs             `OrbatManagerDialog`: side tabs, template bar, search, tree, slot inspector
├── dialog_lifecycle.rs   modal stack registration, Escape, drag cancel, the faction library fetch
├── faction_templates.rs  templates per side; merging a side into a stored faction; save refusals
├── slot_inspector.rs     the "Slot Inspector": the selected slot's role, callsign and rank
├── snapshot.rs           the dialog's read of the document: factions, squads and slot details
├── stats.rs              the tree's total and rendered row counts, published for the gates
├── tree_panel.rs         the tree, windowed once it passes the virtual-row threshold
└── tree_rows.rs          squad and slot rows: rename, add, remove, refile, make squad leader
```

## How it works

The editor page mounts `OrbatManagerDialog`, and the top strip's "ORBAT
Manager" button opens it. On open the dialog takes its overlay z-index from `core::ui::modal_stack`
and fetches the faction library with `GET /api/v1/factions`. Each render reads the document through
`hosted_commands::orbat_manager_snapshot` and shows one side ("BLUFOR", "OPFOR" or "INDFOR"),
filtered by "Search entities..."; the header counts the slots against the server cap of 128 players
and turns to the alert colour above it. The rows and the slot inspector edit through the map
engine's `orbat_*` hosted commands (squads added, renamed and removed; slots added, removed, refiled
and made squad leader; vehicles added from the item
[registry](/documentation/glossary/n_to_z.md#registry)), and the inspector opens the
[Arsenal](/documentation/glossary/a_to_f.md#arsenal) on the selected slot. A tree longer than
`VIRTUAL_SLOT_THRESHOLD` rows renders only the rows in view plus an overscan, and `stats.rs` writes
the counts to `window.__outlinerStats.orbat`.

Templates come from the faction library. "Load Predefined ORBAT…" lists the side's library
factions, and "APPLY TEMPLATE" replaces the side's ORBAT after a confirmation. "Save" re-reads the
selected faction (`GET /api/v1/factions/{id}`), merges the side into it, keeping what the mission
cannot express, refuses a side with no roles and no vehicles, confirms when rows would be dropped,
and writes it with `PUT /api/v1/factions/{id}`; "Save as" creates a faction from the side with
`POST /api/v1/factions`.

## Boundaries

- Depends on: `mission_editing_commands::hosted_commands` (the snapshot and the `orbat_*`
  commands, refiling, `orbat_apply_faction`, `faction_doc_from_side`) and
  `mission_model::slot_line`; the outliner in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/outliner/` (the node model, `flatten_visible`, the
  side filter, the dialog class, the drag latch); the bridge's `entity_selection` and
  `editor_context::open_attributes`/`open_arsenal`; the foundation crates (the
  [API](/documentation/glossary/a_to_f.md#api) client, `AuthStore`, `FactionDoc`, `UserFaction`,
  `RegistryItem`, `modal_stack`, `MaterialIcon`); over HTTP, the faction library of the API's
  [missions](/documentation/glossary/g_to_m.md#missions) domain.
- Used by: the parent module, whose `OrbatManagerDialog`
  `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs` mounts; the headless editor gates in
  `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`, which open the dialog by the
  "ORBAT Manager" label and read its heading, rows and `window.__outlinerStats`;
  `cargo xtask verify editor-orbat-coherency`
  (`tools/checks/repository_checks/src/architecture/editor_orbat_coherency.rs`), which scans the
  parent module and every source file in this folder; the test
  `orbat_manager_overlay_derives_z_from_the_modal_stack` in
  `crates/frontend/foundation/frontend_ui/src/tests/ui.rs`, which reads `dialog.rs`.
- Rules: applying a template changes nothing unless the confirmation is accepted
  (`apply_cancel_noop` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/tests/orbat_manager/roster_and_virtualization.rs`),
  and Escape closes the dialog only while it is the topmost modal;
  no source file here or in the parent module holds the text "Standardization", "IFAK" or
  "Grenade Complement" in any letter case (`cargo xtask verify editor-orbat-coherency`); the
  dialog's overlay z-index comes from `modal_stack::z_class`
  (`orbat_manager_overlay_derives_z_from_the_modal_stack`).

## Related documentation

- [Missions domain](/crates/api/api_missions/src/README.md) — the `/api/v1/factions` routes
  the templates read and write.
- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) — the ORBAT tree and squad authoring.
- [Mission Creator feature inventory: selection](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/selection.md) — selecting from the ORBAT Manager tree.
