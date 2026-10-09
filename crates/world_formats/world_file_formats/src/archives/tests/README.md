# Archive tests

Unit tests of the rkyv archives: that every archive type round-trips byte for byte and refuses corrupted bytes and the bytes of another archive type, and
that the identifier types leave every archive's bytes those of its bare primitive fields.

## Contents

```text
crates/world_formats/world_file_formats/src/archives/tests/
├── archive_round_trip_fixtures.rs  one populated value of each archive type, the round-trip and corruption checks
├── archive_round_trip_tests.rs     each archive's round trip and refusals, empty archives, the `TBDS` version 2 framing
├── archive_wire_identity_tests.rs  each identifier-holding archive serialises to its primitive twin's bytes and reads them back
└── primitive_id_record_shapes.rs   the records with bare `u32` and `String` identifier fields, and each archive's projection onto them
```

## Boundaries

- Depends on: `crate::archives` (the archive types and `codec`), `crate::containers`
  (`TbdsHeader` and the `ContainerHeader` trait) for the satellite framing case, and `crate::ids`.
- Used by: `cargo test -p world_file_formats`; the round-trip and wire identity files are mounted
  from `../mod.rs`, each with a `#[path]` attribute.
- Rules: the tests build their archives in memory and need no asset; a round trip must
  re-serialise to the same bytes, not only to an equal value.
