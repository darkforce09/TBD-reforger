# Mission editing session tests

Unit tests of the editing session's picks: the square slot and circular vehicle hits under a frozen
camera, the document's slot-first tie policy, the marquee order and the squad link inputs; and the
source pin that the picking adapter resolves through the document's mixed slot-and-vehicle queries.

## Contents

```text
crates/mission_editing/mission_editing_session/src/tests/
├── picking_adapter_source.rs  the adapter's mixed pick and marquee calls; the scrubber's cases
├── picking_selection.rs       picks, marquees, the equal-distance tie and the squad link refile
└── source_scrub.rs            blanks comments and literals so a source pin sees only compiled code
```

## Boundaries

- Depends on: `picking` in the crate (both test modules are mounted from `picking.rs` with a
  `#[path]` attribute; `picking_adapter_source.rs` reads `picking.rs` as text),
  `mission_document`, `camera_math`, and `mission_operations::place_orbat` with
  `unit_symbology::squad_links` for the squad link case.
- Used by: `cargo test -p mission_editing_session`.
- Rules: the tests build their camera and documents in memory and need no asset, browser or GPU.
