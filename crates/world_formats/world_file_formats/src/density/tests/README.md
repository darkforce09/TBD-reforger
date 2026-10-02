# Density tile tests

Unit tests of the `TBDD` density codec: the committed Everon tiles against a byte-by-byte
reference decoder, synthetic and unaligned payloads, round trips, refusals and a scan of the
production decoder's source.

## Contents

```text
crates/world_formats/world_file_formats/src/density/tests/
├── tbdd_class_r_scrub.rs     the comment and test-module scrubber the source scan reads `tbdd.rs` through
├── tbdd_parity_reference.rs  the byte-by-byte reference decoder the parity tests compare against
└── tbdd_tests.rs             Everon parity, synthetic shapes, unaligned payloads, header layout, refusals
```

## Boundaries

- Depends on: `crate::density::tbdd`, and the committed tiles under
  `assets/terrains/everon/objects/density/` (Git LFS).
- Used by: `cargo test -p world_file_formats`; each file is mounted from `../tbdd.rs` with a
  `#[path]` attribute.
- Rules: a missing or short Everon corpus fails the run, never skips it; the source scan reads
  `../tbdd.rs` through `include_str!`.
