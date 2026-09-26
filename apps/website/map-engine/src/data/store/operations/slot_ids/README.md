# Duplicate slot id check

Finds [slot](/documentation_v2/glossary/n_to_z.md#slot) ids that a squad of the
[mission](/documentation_v2/glossary/g_to_m.md#mission) document lists more than once, so the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) can refuse a save and name the
squad and the id. It reports and never repairs: the document's ids stay as they are.

## Contents

```text
apps/website/map-engine/src/data/store/operations/slot_ids/
├── duplicates.rs  `duplicate_slot_ids`: the repeated ids, as (callsign, slot id) pairs
├── mod.rs         the module tree; re-exports `duplicate_slot_ids`
└── tests/         unit tests for a hydrated squad that lists one id twice
```

## How it works

`duplicate_slot_ids(doc)` walks every squad's `slotIds` in `small_maps_json` and reports an id the
second time it is seen, provided the slot still exists (`MissionDocCore::slot_exists`), so a
dangling id is never reported. Ids are compared within a label: the squad's callsign, else its
name, else `squad`; that label is the first half of each pair. The document's own writes never
repeat an id in a squad, so a duplicate arrives through a hydrated payload.

## Boundaries

- Depends on: `crate::data::store::MissionDocCore` (`small_maps_json`, `slot_exists`);
  `serde_json`.
- Used by: the Mission Creator's save in
  `apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`, which refuses the save
  before compiling when the list is not empty and shows the pairs through
  `duplicate_slot_id_report` in `apps/website/map-engine/src/editing/commands/merge_report.rs`.
- Rules: a document built by the store's own writes reports nothing, and a hydrated squad that
  lists an existing id twice reports it with its callsign (`test_duplicate_slot_ids` in
  `tests/cases_1.rs`); both source files are on the place path that
  `cargo xtask verify editor-orbat-coherency` scans for `ensure_default_squad`.
