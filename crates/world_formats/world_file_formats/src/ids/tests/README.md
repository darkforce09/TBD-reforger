# Identifier tests

Unit tests of the world file identifier types: the conversions between the two prefab identifier
widths, the row width's `Pod` layout, and the JSON form of every identifier.

## Contents

```text
crates/world_formats/world_file_formats/src/ids/tests/
└── identifier_accessor_tests.rs  prefab id widening and checked narrowing, the two-byte row id, bare-value JSON of every id
```

## Boundaries

- Depends on: `crate::ids`, `bytemuck`, and `serde_json` (a dev-dependency).
- Used by: `cargo test -p world_file_formats`; the file is mounted from `../mod.rs` with a
  `#[path]` attribute.
- Rules: the tests need no asset; a narrowing that does not fit is an error, never a truncation.
