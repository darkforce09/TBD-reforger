# ORBAT slot ids

The `orbat_slot_ids` crate: the two identifiers of one ORBAT slot. `SlotUid` is the slot's
durable editor id (`slots[].uid`), the key the Mission Creator's mission document stores each slot
row under; `SlotId` is the slot's derived wire id (`slots[].id`,
`faction:callsign:role:occurrence`), which the compiled mission document names it by and which
shifts under role renames and reorders.

## Contents

```text
crates/foundation/orbat_slot_ids/
├── Cargo.toml  the package: `newtype_ids`, layout tier 1
└── src/        the two id types and the prelude
```

## How it works

Both types are `newtype_ids::string_id!` declarations: an owned `String` that serialises and
deserialises as the bare string, compares and hashes as that string, and borrows as `&str`. A
field or parameter that holds a slot's editor id is typed `SlotUid` and one that holds the
compiled wire id is typed `SlotId`, so the compiler rejects passing one where the other belongs
while every serialized byte stays what the plain string wrote.

## Getting started

Run from the repository root:

```bash
cargo build -p orbat_slot_ids   # the crate holds no tests; its callers test the wire form
```

## Configuration

No feature and no environment variable.

## Public surface

- `SlotUid` and `SlotId`, at the crate root and in `prelude`, each with the `string_id!` surface
  (`new`, `as_str`, `into_inner`, `Display`, `From<String>`, `From<&str>`, `FromStr`,
  `Borrow<str>`, `AsRef<str>`, equality with `str`).

## Boundaries

- Depends on: `newtype_ids` (the `string_id!` macro).
- Used by: `mission_model` (the compiled slot, vehicle seat and squad rows and the VIP win rule),
  `mission_document` (the editor's slot rows), `mission_operations` (the authoring operations
  over those rows), `mission_compiler` (the game-document substitutions) and `unit_symbology`
  (the squad link inputs).
- Rules: foundation tier, so the crate depends on foundation crates only (`cargo xtask verify
  crate-tiers`); both ids stay serde-transparent.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
