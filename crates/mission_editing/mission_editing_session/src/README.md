# Mission editing session source

The source of `mission_editing_session`: the editing host, the undo drive, undo grouping, the
subject router, the selection universe, the picks, the overlay lanes and the crate root.

## Contents

```text
crates/mission_editing/mission_editing_session/src/
├── batch.rs               `with_batch`: several document transactions as one undo step
├── history/               undo, redo and the post-change tail the host installs
├── host.rs                `EditingHost`: the live document, the selection and the id minter
├── lanes/                 connection, comment and marker lanes from the document, and their picks
├── lib.rs                 the crate root: module header and `mod` lines
├── picking.rs             slot and vehicle picks and marquees mapped to document ids, squad link inputs
├── prelude.rs             the names most readers import
├── routing.rs             `route_target`: the surface that owns a subject, and whether a click reaches it
├── selection_universe.rs  selectable ids, the crew hide, the map-render slots, the paste anchor
└── tests/                 unit tests for the picks, the marquees and the document's tie policy
```

## How it works

`host.rs` holds the thread-local host; `batch.rs` and `history/` reach the document only through
it. `routing.rs` reads a zone's centre through `selection_universe::zone_centre`, which stays
private to the crate. `picking.rs`, `selection_universe.rs` and `lanes/` are pure over the
document's JSON, its slot columns and a frozen camera. `picking.rs` mounts its tests from `tests/`.

## Boundaries

- Depends on: the crates named in the [crate README](../README.md).
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: every public entry point that reads the document opens one borrow and drops it before
  returning; nothing here touches a browser or a GPU.
