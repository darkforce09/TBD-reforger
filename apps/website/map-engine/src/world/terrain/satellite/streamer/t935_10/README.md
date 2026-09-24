# Satellite container reader tests

Unit tests of the satellite container reader, kept as a test module of the parent folder: they
frame one small pyramid (a 5 × 5 base cut into 2-pixel tiles) both as a version 1 container with a
JSON index and as a version 2 container with an archived index, and hold the two readers to the
same answer.

## Contents

```text
apps/website/map-engine/src/world/terrain/satellite/streamer/t935_10/
├── cases_1.rs  the cases: both versions agreeing, the refusals, and Everon's tiling derivation
└── mod.rs      the synthetic pyramid, its version 1 and version 2 framing, and the shared helpers
```

## Boundaries

- Depends on: the parent module, `crate::world::terrain::satellite::streamer` (the parsers, the
  level picks, `mip_dims` and `tile_rect`); `crate::io::archives::satellite` (the archived index
  types) and `crate::io::archives::codec` (its serialiser); `crate::io::containers` (the `TBDS`
  header).
- Used by: nothing; the parent declares the module only under `cfg(test)`.
- Rules: the cases hold that a 12-byte prefix sizes the header and index of either version and a
  third version is refused by number (`index_range_end_sizes_v1_and_v2_from_a_twelve_byte_prefix`);
  both versions of one pyramid describe the same tiles and point at the same bytes
  (`v1_and_v2_of_one_pyramid_read_back_identical`); version 2 reports the terrain id and world
  bounds it does not carry as absent (`v2_reports_the_fields_it_does_not_carry_as_absent`); an
  index one byte short, a corrupt index, a tile grid that contradicts `tile_px` and an unknown tile
  format are refused (`an_index_len_one_byte_short_is_an_error`, `a_corrupt_index_is_an_error`,
  `a_grid_that_contradicts_tile_px_is_rejected`, `an_unknown_tile_format_is_rejected`); an
  unaligned buffer still validates (`an_unaligned_buffer_still_validates`); and the tile
  derivation gives Everon's 12 800² base 14 levels and, with 8192-pixel tiles, four tiles at
  level 0 and 17 in all (`the_derivation_reproduces_the_committed_everon_tiling`).
