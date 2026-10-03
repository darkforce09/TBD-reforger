# Mission wire safety tests

The unit tests of `mission_wire_safety`: the byte rule, the name scan's payload paths and reporting
caps, and the cargo capacity walk against a catalog.

## Contents

```text
crates/mission/mission_wire_safety/src/tests/
├── cases_1.rs  13 cases: byte rule, scanned names, collapsing and capping, capacity lines
└── mod.rs      the shared catalog and one-slot payload fixtures; mounts the case file
```

## Boundaries

- Depends on: the crate root's public items and `serde_json`.
- Used by: `cargo test -p mission_wire_safety`.
- Rules: test files stay at or under 1000 lines; a case is moved, never deleted or loosened.
