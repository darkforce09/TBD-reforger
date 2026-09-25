# Loadout serialisation and transfer

The pure loadout core of the [arsenal](/documentation_v2/glossary.md#arsenal): reading and writing
one [slot](/documentation_v2/glossary.md#slot)'s persisted `SlotLoadoutV2` JSON, the downloadable
loadout export and its import gate, the copy buffer that applies loadouts across a selection, and
the receipts and refusals the Arsenal tab shows.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/arsenal/loadout/
├── attachments_and_faults.rs       packed attachment sets; the loadout verdict and kit evidence
├── buffered_loadout_operations.rs  Copy, Apply and Remove Everything: plans, seeded draw, receipts
├── loadout_export.rs               `try_export`: the export gate, refusing cargo over capacity
├── loadout_import.rs               `try_import`: JSON, schema and rule checks before a pick applies
└── slot_loadout_serialization.rs   `SlotLoadoutV2` to picks and back; the export document writer
```

## How it works

The Arsenal edits a picks map: one [registry](/documentation_v2/glossary.md#registry) resource
name per loadout row key, plus a synthetic `attachments@<weapon>` key per weapon whose value packs
that weapon's attachment set with the U+001F separator, which no resource name can contain.
`loadout_to_picks` reads a slot's `SlotLoadoutV2` JSON into that map, and `picks_to_loadout`
writes it back in canonical form, with weapons by slot index, wear by key, and a `cargo` key only
once the author has touched cargo; a present empty `cargo` array records a deliberate clear, so
the default cargo is not seeded again.

The export and import gates share one rule set. `try_export` refuses while any cargo is over the
catalogued capacity of the garment that carries it, and otherwise writes the export document with
`picks_to_export`, stamped with the catalog's `modpackId`. `try_import` checks the file in order:
valid JSON, the shipped loadout-export schema, then the compatibility, attachment and capacity
rules; any failure returns only refusals, and a foreign `modpackId` only warns. Cargo in a
container that no picked garment wears blocks no export, import or Apply; it only fails the
verdict that `loadout_faults` gives the Arsenal's badge.

The copy buffer lives in the map engine. `plan_apply` draws one buffered loadout per selected
target with `buffer_draw`, a SplitMix64 draw that is uniform, independent per target and
reproducible from the session seed; the plan is all or nothing and passes the same rules as an
import. `plan_remove` writes `stripped_loadout`, an empty loadout with `cargo: []`.
`commit_one_write` runs the history tail only when the document acknowledges the write, and the
Apply and Remove Everything receipts count the writes the document took, not the plan.

## Boundaries

- Depends on: the parent `apps/website/frontend/src/v2/apps/editor/arsenal/loadout.rs` (the
  kind-sourced loadout rows), the rules in `apps/website/frontend/src/v2/apps/editor/arsenal/rules/`
  (validation, capacity, the export schema check, `CargoRow`), `RegistryItem` from
  `crate::v2::core::api::dto`, `website_map_engine::data::store::operations::cargo`
  (`BufferedLoadout`, `LoadoutWrite`, `commit_writes`) and, in the browser build,
  `bridge::host_state::editor_context::slots_json` for the slot's character prefab.
- Used by: `loadout.rs`, which re-exports the public items; in
  `apps/website/frontend/src/v2/apps/editor/arsenal/`, `mod.rs` (the Arsenal tab),
  `loadout_commands.rs` (the document writes) and `tab_content.rs` with its `tab_content/`
  (import, export, Copy, Apply and Remove Everything); the Arsenal panels in
  `apps/website/frontend/src/v2/apps/editor/ui/arsenal/panels.rs`, for the attachment sets.
- Rules: a refused write mints no undo step and leaves the
  [mission](/documentation_v2/glossary.md#mission) clean
  (`a_refused_write_mints_no_tail_and_does_not_dirty_the_mission`); a document that fails a check
  applies nothing (`a_document_that_does_not_validate_applies_nothing`); Apply uses the import gate
  (`the_apply_gate_is_the_import_gate`); the draw is uniform, independent and reproducible
  (`the_draw_is_uniform_independent_and_reproducible`); the export satisfies the shipped schema
  (`exported_file_satisfies_the_v2_branch_of_the_real_schema`). The tests live in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tests/loadout/`.

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Arsenal tab's loadout edits, import and export.
