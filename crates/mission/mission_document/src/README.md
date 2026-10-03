# Mission document source

The source of `mission_document`: the document's rows, its selection policy, its ids and error, and
the tests that build a whole document.

## Contents

```text
crates/mission/mission_document/src/
├── error.rs          `Error` and `Result`: the refusals of `MissionDocCore::apply_update`
├── ids.rs            the document-only ids: faction, squad, layer, entity, vehicle, comment, composition, connection, crew seat, client
├── lib.rs            the crate root: module header, `mod` lines and re-exports
├── prelude.rs        the document, its rows, patches, ids and error for glob import
├── rows/             `MissionDocCore`: root maps, writes, reads, hydrate, merge, materialise, undo
├── selection.rs      the pick, marquee and selection policy over the slot columns
├── test_fixtures.rs  `SteppingClock`, the undo clock of the crate's tests and of `test_fixtures` users
└── tests/            whole-document tests: id array merges, undo groups, payload and vehicle round trips, prelude surface
```

## How it works

`lib.rs` re-exports `MissionDocCore` and the rows, patches and connection vocabulary its methods
take; `rows/` holds the `impl MissionDocCore` blocks by concern and `selection.rs` the picking and
marquee policy. A command takes a slot's durable editor id (`orbat_slot_ids::SlotUid`), the
mission model's ids (`ZoneId`, `TriggerId`, `MarkerId`, `MissionId`), `mission_validation::AssetId`
and this crate's ids, and writes their bare
strings into the maps, so the Yjs maps and every exported payload keep their exact shapes.
`MissionDocCore::with_undo_clock` is the host clock seam (a `time_source::Clock`); `new` reads the
platform clock, or `SteppingClock` in this crate's unit tests.

## Boundaries

- Depends on: `mission_crdt`, `mission_model`, `mission_validation`, `newtype_ids`,
  `time_source`, `yrs`, `serde_json`, `thiserror`.
- Used by: the map engine's document store, operations and editing layer, and the Mission
  Creator through them.
- Rules: the tests that need the payload compiler or the game-document compiler live in
  `tests/` with those crates as dev-dependencies; nothing here reads a map engine module.
