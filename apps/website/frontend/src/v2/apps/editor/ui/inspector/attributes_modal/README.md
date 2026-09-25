# Attributes dialog parts

The tabs, fields and writes of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
Attributes dialog, which edits one or several placed [slots](/documentation_v2/glossary.md#slot), or
one placed vehicle. The dialog itself, `AttributesModal`, lives in the parent module
`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal.rs`, which declares these
modules.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/
├── asset_type_picker.rs               the "Type" picker: a searchable asset catalog tree
├── attribute_commits_and_revert.rs    position and identity writes to the targets; Revert
├── faction_and_squad_reassignment.rs  the "Faction" and "Squad" pickers that refile targets
├── field_gates_and_labels.rs          `MultiOpts` and `Gate`: "Apply to all" opt-ins, locked fields
├── field_inputs.rs                    number and text drafts that commit on blur or Enter
├── identity_tab.rs                    the Identity tab: type, role, description, tag, faction, squad
├── spatial_transform_tab.rs           the Transform tab: X, Y, Z, rotation and stance
└── vehicle_attributes.rs              the vehicle view: heading, cargo rows and crew seats
```

## How it works

`AttributesModal` opens on the id that the bridge's `editor_context::open_attributes` sets: a
double-click on the map, a double-click on an outliner or
[ORBAT](/documentation_v2/glossary.md#orbat) Manager row, or the context menu. `open_arsenal`, from
the context menu and the ORBAT Manager's slot inspector, opens it on the
[Arsenal](/documentation_v2/glossary.md#arsenal) tab. The dialog snapshots every target on open and
re-reads its fields on each document change, so an undo taken while it is open refreshes them. Its
tabs are "Transform", "Identity", "States" (a placeholder) and "Arsenal", which mounts the Arsenal
tab of `apps/website/frontend/src/v2/apps/editor/arsenal/` for the id the dialog opened on.

- **Fields.** `field_inputs.rs` keeps a draft while a field has focus and commits it on blur or
  Enter when the value is finite and new; Escape puts the stored value back. Arrow and Page keys
  nudge a number by 1, or by 0.1, 10 or 100 with Ctrl, Shift or Alt held.
- **Multi-edit.** Over a multi-selection every write goes to all target slots at once. A field
  whose values differ across the selection shows blank and stays locked until its "Apply to all"
  box is ticked (`Gate::maybe` over a `MultiOpts` latch); the subtitle counts the slots and says
  when vehicles in the selection are left out.
- **Writes.** The transform and identity fields write through the map engine's hosted attribute
  commands (`attrs_update_position`, `attrs_update_slot` and their multi-target forms), the
  faction and squad pickers through `reassign_slots`; on a locked layer the Transform fields
  refuse and the tab says why. "Revert" writes every target's snapshot back, its squad included.
- **Vehicles.** A vehicle id opens `vehicle_attrs_view`: the heading, "Cargo" rows from the item
  [registry](/documentation_v2/glossary.md#registry)'s carriable kinds, and "Crew" seats (driver,
  gunner, commander and four cargo seats) filled from the placed slots, each written through the
  map engine's vehicle commands.

## Boundaries

- Depends on: `website_map_engine::editing::hosted_commands` (the attribute reads and writes, the
  squad reassignment, the vehicle commands) and `website_map_engine::data::store::operations::reassign`;
  the Arsenal's `asset_catalog` in `apps/website/frontend/src/v2/apps/editor/arsenal/` for the type
  picker's tree and search; `editor_context::close_attributes` in the bridge; `MaterialIcon` and the
  `RegistryItem` DTO from `crate::v2::core`.
- Used by: the parent module only.
- Rules: a field writes only a new finite value (`should_commit_writes_only_a_new_finite_value` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/attributes_modal/numeric_field_input.rs`);
  the faction and squad pickers move the whole selection in one undo group
  (`the_picker_commits_the_whole_selection_in_one_undo_group` in
  `batch_faction_and_squad_reassignment.rs` beside it).

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Attributes dialog and its tabs, entry by entry.
- [Eden attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
  — the Arma 3 Eden attributes the dialog is measured against.
