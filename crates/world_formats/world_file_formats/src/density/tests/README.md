# Density tile tests

Unit tests of the `TBDD` density codec: synthetic and unaligned payloads against a byte-by-byte
reference decoder, the header layout, round trips and refusals.

## Contents

```text
crates/world_formats/world_file_formats/src/density/tests/
├── tbdd_parity_reference.rs  the byte-by-byte reference decoder the parity tests compare against
└── tbdd_tests.rs             synthetic shapes, unaligned payloads, header layout, round trips, refusals
```

## Boundaries

- Depends on: `crate::density::tbdd`.
- Used by: `cargo test -p world_file_formats`; each file is mounted from `../tbdd.rs` with a
  `#[path]` attribute.
- Rules: the tests build their payloads in memory and need no asset.
