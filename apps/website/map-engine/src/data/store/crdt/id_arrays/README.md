# Ordered id arrays

The ordered id lists of the [mission](/documentation_v2/glossary/g_to_m.md#mission) document: a squad's
`slotIds` and a layer's `entityIds` held as native `yrs` arrays, so that two peers appending at the
same moment both keep their id, and the helpers that read, append, remove and move ids in that
form or as a plain array.

## Contents

```text
apps/website/map-engine/src/data/store/crdt/id_arrays/
├── mission_doc_tests/  unit tests that run the arrays through a whole `MissionDocCore`
├── mod.rs              the module tree; re-exports everything public in `native_arrays.rs`
├── native_arrays.rs    the two native field names, the readers and writers, the hydrate promotion
└── tests/              unit tests for both forms on bare `yrs` maps, promotion, concurrent appends
```

## How it works

Exactly two id lists are native: `SLOT_IDS` (`"slotIds"`, on a row of the `squads` root map) and
`ENTITY_IDS` (`"entityIds"`, on a row of `editorLayers`). Every other id list, such as a faction's
`squadIds` or a squad's `vehicleIds`, is an opaque `Any::Array` value that a write clones and
replaces whole. A replaced value is last-writer-wins, so two peers appending to the same plain
list lose one id; a native `push_back` merges, so both survive.

| Helper | Native field | Plain field |
|---|---|---|
| `read_id_array`, `read_field`, `read_field_ids`, `read_ids` | read in order | read in order |
| `append_id` | promoted to native first, then `push_back`; an id already present is skipped | cloned, appended, replaced; an id already present is skipped |
| `retain_in` | matching entries deleted in place, back to front; a still-plain value is filtered and promoted | cloned, filtered, replaced |
| `move_id` | one id moved to an index, clamped | left alone |
| `insert_empty_native`, `replace_native` | a fresh native array replaces the value | a fresh native array replaces the value |

`retain_ids` filters a cloned plain list and serves the opaque lists; a native field always goes
through `retain_in`, so the live array is edited in place. A reader returns an empty list when the
row or the field is missing, and `append_id` does nothing when the row is missing.
`migrate_legacy_id_lists` runs once per hydrate: it turns every plain `slotIds` in `squads` and
`entityIds` in `editorLayers` into a native array in the same order, and leaves a missing key
missing, so a payload that omitted a list does not gain an empty one on the wire.

## Public surface

- `SLOT_IDS` and `ENTITY_IDS`: the two native field names, which the row module of
  `apps/website/map-engine/src/data/store/rows/` writes and reads.
- `append_id`, `insert_empty_native`, `replace_native`, `retain_in`, `retain_ids`,
  `read_id_array`, `read_field_ids` and `migrate_legacy_id_lists`: the id list operations the row
  module's [slot](/documentation_v2/glossary/n_to_z.md#slot), squad, layer, vehicle, composition, paste,
  hydrate, merge and materialize code runs inside its own transactions.
- `read_field`, `read_ids`, `is_native_array` and `move_id`: public, with callers in this folder
  only (`read_id_array` and the tests).

## Boundaries

- Depends on: `yrs` (`Any`, `Array`, `ArrayPrelim`, `MapRef`, `Out`, `ReadTxn`,
  `TransactionMut`) and nothing else of the crate.
- Used by: `crate::data::store::rows` (the row module's slot, squad, layer, vehicle, composition
  and paste writes, `hydrate`, the merge, `materialize`, the lookups and formations); nothing
  outside the document store.
- Rules:
  - only `slotIds` and `entityIds` are native, and an append never adds an id twice
    (`read_append_retain_move_over_yarray` in `tests/cases_1.rs`);
  - a plain list and a native list read the same ids in the same order
    (`both_forms_read_identically`), and the hydrate promotion keeps that order
    (`hydrate_migration_promotes_legacy_any_array`);
  - concurrent native appends both survive a merge while a clone-and-replace loses one
    (`concurrent_yarray_appends_both_survive` and `concurrent_any_array_clone_rewrite_drops_an_id`
    in `tests/cases_1.rs`, `two_peers_concurrent_slot_id_appends_both_survive` in
    `mission_doc_tests/cases_1.rs`).
