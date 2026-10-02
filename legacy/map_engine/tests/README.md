# Map engine integration suites

The crate-root integration suites of the map engine: three headless document suites that run
editor operations, paste and zone save and reload against the mission document, through the same
headless boundary the Mission Creator and the API use.

## Contents

```text
legacy/map_engine/tests/
├── operation_boundaries.rs    editor operations at the edges of the document
├── paste_keeps_authored_z.rs  paste keeps each authored height
└── zone_round_trip.rs         zones survive a save and a reload
```

## Boundaries

- Depends on: the crate's `store` tier (the suites are gated on it).
- Used by: `cargo test -p map_engine --all-features`.
- Rules: the suites keep their assertions and fixtures. The camera suites run in their own crate
  (`crates/geometry/camera_math/tests/`).
