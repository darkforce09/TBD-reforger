# Id array tests through a whole document

Unit tests that run the squad and layer id arrays through a whole
[mission](/documentation_v2/glossary/g_to_m.md#mission) document, `MissionDocCore`, rather than a bare
`yrs` map: two peers appending at once, the undo of one peer's append, and a hydrate whose id
lists arrive as plain JSON arrays.

## Contents

```text
apps/website/map-engine/src/data/store/crdt/id_arrays/mission_doc_tests/
├── cases_1.rs  the three cases: concurrent appends, local-only undo, hydrate promotion
└── mod.rs      the module tree; the fixtures: two seeded peers, id readers, a native probe
```

## Boundaries

- Depends on: the parent module's `is_native_array`, `SLOT_IDS` and `ENTITY_IDS`;
  `crate::data::store::MissionDocCore` (`with_client_id`, `add_faction`, `add_squad`,
  `add_editor_layer`, `add_slot`, `apply_update`, `encode_state`, `undo`, `hydrate`,
  `small_maps_json`); `yrs` to decode `encode_state` into a probe document; `serde_json`.
- Used by: nothing outside the folder;
  `apps/website/map-engine/src/data/store/crdt/id_arrays/mod.rs` compiles it only in test builds
  (`#[cfg(test)] mod mission_doc_tests;`).
- Rules:
  - two peers that append to one squad and one layer at once both keep their id after exchanging
    updates, and converge on the same list (`two_peers_concurrent_slot_id_appends_both_survive`);
  - undo takes back only the local append and leaves the peer's
    (`undo_removes_only_the_local_append`);
  - a hydrated plain `slotIds` or `entityIds` list becomes a native array in the same order
    (`hydrate_legacy_payload_migrates_slot_ids_to_yarray`);
  - each peer is built with its own fixed client id (`seed_peers`), because `apply_update`
    refuses an update that carries more edits under the receiving document's own client id than
    that document has made, once it has made one.
