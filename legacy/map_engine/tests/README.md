# Map engine integration suites

The crate-root integration suites of the map engine: three headless document suites that run the
authoring commands, paste and zone save and reload against the mission document, through the same
mission crates the `editing` tier hosts for the Mission Creator.

## Contents

```text
legacy/map_engine/tests/
├── operation_boundaries.rs    authoring commands at the edges of the document
├── paste_keeps_authored_z.rs  paste keeps each authored height
└── zone_round_trip.rs         zones survive a save and a reload
```

## Boundaries

- Depends on: the crate's `editing` tier (the suites are gated on it), which links
  `mission_document`, `mission_operations`, `mission_payload` and `formation_geometry`.
- Used by: `cargo test -p map_engine --all-features`.
- Rules: the suites keep their assertions and fixtures. The camera suites run in their own crate
  (`crates/geometry/camera_math/tests/`).
