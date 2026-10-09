# ORBAT slot ids source

The source of `orbat_slot_ids`: the two slot id declarations and the crate root that exports them.

## Contents

```text
crates/foundation/orbat_slot_ids/src/
├── lib.rs        the crate root: module header, `mod` lines and the re-exports
├── prelude.rs    `SlotId` and `SlotUid` for glob import
└── slot_ids.rs   `SlotUid`, the durable editor id, and `SlotId`, the derived wire id
```

## How it works

`slot_ids.rs` declares both types with `newtype_ids::string_id!`; `lib.rs` re-exports them at the
crate root and `prelude.rs` for glob import.

## Boundaries

- Depends on: `newtype_ids`.
- Used by: the mission crates and `unit_symbology`, through the crate root.
- Rules: no other module; a new slot identifier belongs here only when it names a slot.
