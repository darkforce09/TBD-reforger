# Vegetation density tiles

The `TBDD` codec: one terrain chunk's vegetation density grid, the per-corner tree and rock
counts the map's forest mass is drawn from, encoded as a 16-byte header and `u16` planes.

## Contents

```text
apps/website/map-engine/src/io/density/
├── mod.rs   the module tree
├── tbdd.rs  `TbddHeader`, `TbddGrid`, `TbddError`, `decode_tbdd` and `encode_tbdd`
└── tests/   unit tests: committed Everon tiles, a reference decoder, round trips and a source scan
```

## How it works

A `TBDD` buffer is little-endian: the header (`magic` `TBDD`, `version`, `cell_m`, `cols`,
`rows`, `channel_count` and three zero bytes, 16 bytes in all), then `channel_count` planes of
`cols × rows` `u16` values, row-major, in `DENSITY_CHANNEL_NAMES` order (`tree`, then `rock`).
The committed tiles hold two 65 × 65 planes at 8 m, the shared-border corners of a 512 m chunk.

`decode_tbdd` reads the header unaligned and fails with `Short` under 16 bytes, `BadMagic` on a
wrong magic and `Truncated` when the planes do not fit; bytes past the planes are ignored, and the
version is returned in `TbddGrid.version` without a check. The payload is borrowed as `u16` when
it is 2-byte aligned and copied once when not. `encode_tbdd(cell_m, cols, rows, channels)` always
writes version 1 and panics when a channel's length is not `cols × rows` or there are more than
255 channels.

## Boundaries

- Depends on: `bytemuck`.
- Used by:
  - `crate::world::environment::vegetation::loader`, which fetches
    `objects/density/{cx}_{cy}.bin` for each chunk and skips a tile that fails to decode;
  - `crate::streaming::loaders`' store tests;
  - the developer tools: the world export writes tiles
    (`tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/`), and the export
    validation, the mathematical verification and the object goldens decode or rebuild them
    (`tools_v2/developer-tools/src/world_export_pipeline/` and
    `tools_v2/developer-tools/src/map_verification/object_goldens/`).
- Rules:
  - the header stays 16 bytes and 2-aligned on a little-endian target (compile-time assertions in
    `tbdd.rs`; `header_pod_is_the_on_disk_header` in `tests/tbdd_tests.rs`);
  - every committed tile under `assets_v2/terrains/everon/objects/density/` decodes exactly as the
    byte-by-byte reference decoder in `tests/tbdd_parity_reference.rs` does
    (`everon_tiles_decode_bit_identically_to_the_old_loop`), aligned or not
    (`unaligned_payload_decodes_identically`);
  - the production decoder has no per-byte loop:
    `production_decode_has_no_per_byte_assembly_loop` scans `tbdd.rs` with the comment and test
    scrubber in `tests/tbdd_class_r_scrub.rs`;
  - `tools_v2/developer-tools/src/world_export_pipeline/vegetation_density.rs` keeps its own copy
    of the header size, the channel names and the version, so a change here changes it too.

## Related documentation

- [Terrain assets](/assets_v2/terrains/README.md) — the served terrain tree that holds the tiles.
