# Mission editing session tests

Unit tests of the editing session's picks: the square slot and circular vehicle hits under a frozen
camera, the document's slot-first tie policy, the marquee order and the squad link inputs.

## Contents

```text
crates/mission_editing/mission_editing_session/src/tests/
└── picking_selection.rs  picks, marquees, the equal-distance tie and the squad link refile
```

## Boundaries

- Depends on: `picking` in the crate (mounted from `picking.rs` with a `#[path]` attribute),
  `mission_document`, `camera_math`, and `mission_operations::place_orbat` with
  `unit_symbology::squad_links` for the squad link case.
- Used by: `cargo test -p mission_editing_session`.
- Rules: the tests build their camera and documents in memory and need no asset, browser or GPU.
