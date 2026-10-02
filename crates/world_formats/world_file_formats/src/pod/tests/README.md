# Object row tests

Unit tests of `ObjectInstancePod`: its size, alignment and field offsets, and the zero-copy casts
to and from `TBDC` payload bytes.

## Contents

```text
crates/world_formats/world_file_formats/src/pod/tests/
└── instance_tests.rs  size, named bytes, field offsets, primitive-id byte identity, byte round trip, truncated, misaligned and empty payloads
```

## Boundaries

- Depends on: `crate::pod::instance`, `crate::ids::InstancePrefabId` and
  `crate::archives::codec::BinaryError`.
- Used by: `cargo test -p world_file_formats`; the file is mounted from `../instance.rs` with a
  `#[path]` attribute.
- Rules: the tests need no asset; a truncated or misaligned payload must be an error, never a panic.
