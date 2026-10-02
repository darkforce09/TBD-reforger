# Container header tests

Unit tests of the four fixed-header containers: sizes, wire layouts, round trips, bathymetry level
spans and every refusal path.

## Contents

```text
crates/world_formats/world_file_formats/src/containers/tests/
├── container_header_fixtures.rs  `AlignedBuf` and `framed`: a header and its payload in one 4-aligned buffer
└── container_header_tests.rs     `TBDC`, `TBDE`, `TBDB` and `TBDS`: layouts, round trips, spans, refusals
```

## Boundaries

- Depends on: `crate::containers` (the header trait and the four headers),
  `crate::archives::codec::BinaryError` and `crate::pod::instance` (the `TBDC` row).
- Used by: `cargo test -p world_file_formats`; both files are mounted from `../mod.rs` with a
  `#[path]` attribute.
- Rules: the tests build their buffers in memory and need no asset; a short, misnamed, wrongly
  versioned, wrongly sized or misaligned buffer must be an error, never a panic.
