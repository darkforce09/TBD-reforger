# Payload compiler tests

Unit tests of the payload compiler: the save and export payload shapes, row order, `payloadExtras`,
the version body, and the round trip of every authored extension block through the compiler.

## Contents

```text
crates/mission/mission_payload/src/tests/
├── cases_1.rs                 the save and export shapes, row order, extras, titles and the version body
├── extension_round_trips.rs  one table row per authored block, compiled and read back through the registry
└── mod.rs                    the shared document fixtures and the briefing prose constants
```

## Boundaries

- Depends on: the crate root and `mission_model::authored_blocks`.
- Rules: the round trip of a briefing through the CRDT store lives with the store
  (`crates/mission/mission_document/src/tests/payload_round_trips.rs`), since this crate never
  depends on the store.
