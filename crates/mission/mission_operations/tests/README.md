# Mission operations integration suites

The integration suites of `mission_operations`: each drives the authoring commands through the
crate's public surface against a whole [mission](/documentation/glossary/g_to_m.md#mission)
document, the way the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) does,
and checks what the document, its undo history and its compiled payload hold afterwards.

## Contents

```text
crates/mission/mission_operations/tests/
├── operation_boundaries.rs  commands at the document's edges: refused transforms, loadout copies, clipboard paste and its authored heights
└── zone_round_trip.rs       zone geometry survives a payload compile and a re-hydrate
```

## How it works

Each suite builds its document in memory with `MissionDocCore` and calls the public commands of
`mission_operations` (`transform`, `cargo`, `entity`) or the document's own mutators. The zone
suite compiles the document with `mission_payload::compile_payload`, hydrates a fresh document
from the payload and compares the two; the arrange checks name their edges with
`formation_geometry::AlignEdge`.

## Boundaries

- Depends on: the crate's public surface and its regular dependencies `mission_document`,
  `mission_payload`, `formation_geometry` and `serde_json`; no fixture files.
- Used by: `cargo test -p mission_operations`.
- Rules: a refused command leaves the document and its undo history unchanged
  (`refused_transform_leaves_document_and_history_unchanged`); a paste keeps each authored height
  (`a_multi_slot_paste_gives_each_copy_its_own_source_elevation`); zones survive a save and a
  reload (`zone_geometry_survives_save_and_reload`).
