# Mission wire safety source

The source of `mission_wire_safety`: the byte rule, the two scans of a mission editor payload, and
the crate root that exports them.

## Contents

```text
crates/mission/mission_wire_safety/src/
├── lib.rs      the crate root: module header, `mod` lines and the re-exports
├── prelude.rs  every public item for glob import
├── scan.rs     the control-character scan of authored names and the cargo capacity scan
└── tests/      unit tests of the byte rule, the reporting caps and the capacity walk
```

## How it works

`lib.rs` re-exports the public items of `scan.rs` at the crate root and in `prelude`. `scan.rs`
keeps its reporting helpers private to the crate: the destination of each scanned name, the
per-value finding counts and the garment lookup that lets `armoredVest` stand in for `vest`.

## Boundaries

- Depends on: `serde_json`.
- Used by: the API and the map engine, through the crate root.
- Rules: `CARGO_CONTAINERS` in `scan.rs` is a hand copy of the single-page app's list in
  `crates/frontend/workspaces/mission_creator_state/src/arsenal_rules/cargo_capacity_and_delivery.rs`; no test holds
  the two equal.
