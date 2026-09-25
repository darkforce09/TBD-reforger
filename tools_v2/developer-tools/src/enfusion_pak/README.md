# Enfusion pak archive access

Reads the game's `.pak` archives, the `FORM`/`PAC1` files an
[Enfusion](/documentation_v2/glossary.md#enfusion) install ships under `addons/`, and loose
extracted folders, behind one virtual file system. One parser and one decompressor serve two
consumers with different rules: the building-blueprint compiler and the world-export and map
tooling.

## Contents

```text
tools_v2/developer-tools/src/enfusion_pak/
├── archive_reader.rs      `PakIndex` and `PakEntry`: bounded chunk and directory parse, payload reads
├── loose_source.rs        `DirSource` for extracted folders and `LayeredSource`, the ordered fallback
├── mod.rs                 the module tree and `ReadPolicy`; re-exports the public types
├── payload.rs             entry decompression under each consumer's codec and length rules
├── tests/                 unit tests for both policies, synthetic archives and the real-install checks
├── virtual_filesystem.rs  `AssetSource`, `PakSet` and `normalize_path`: merged archives and path lookup
└── world_source.rs        `PakVfs`, the world tooling's facade: game-root discovery and raw entry access
```

## How it works

`parse_archive` checks the `FORM` magic and the `PAC1` type, walks the big-endian chunk headers to
the `DATA` and `FILE` chunks, and reads the `FILE` directory tree iteratively (a stack, not
recursion), so a malformed tree cannot exhaust the stack. Each file record gives an absolute file
offset, the stored and decompressed lengths, a six-byte method tag and a compressed flag.

`PakSet` opens every `.pak` in a folder in sorted name order and merges their directories; the first
archive to hold a path wins, and later archives never shadow it. `ReadPolicy` sets the rest:

| Rule | `Blueprint` (`PakSet::from_dir`) | `World` (`PakVfs::open`) |
|---|---|---|
| Path lookup | ASCII case folded, `\` to `/`, leading `/` dropped (`normalize_path`) | case kept, `\` to `/`, outer and repeated `/` dropped |
| Entry bounds | every entry must lie inside the `DATA` chunk | not checked |
| Decompression | zlib, and the output length must equal the directory's | zlib, then raw deflate when zlib fails |
| Malformed archive | the whole set fails to open | skipped with a message on stderr |

`PakVfs::open` reads `<game>/addons/`; `PakVfs::open_default` takes the game folder from
`ENFUSION_GAME_PATH`, or `$HOME/.cache/enfusion-mcp-root` when it is unset, and
`PakSet::default_dir` is that cache's `addons/`. `PakVfs` also exposes the raw stored bytes, the
`DATA` offset, the method tag and every path, which the texture and topo decoders use for diagnosis.

`DirSource` resolves a path exactly first, then case-insensitively one component at a time.
`LayeredSource` asks each source in order and reads from the first that holds the path, so a missing
file falls through to the next source but a read error in the chosen source does not.

## Boundaries

- Depends on: `flate2` for zlib and raw deflate, and `anyhow`; nothing else in the crate.
- Used by:
  - `crate::blueprint` (`archive_emission::library_reader`, `bvh::batch_processing`,
    `bvh::prefab_catalog`, `mesh_decoding::archive_inspection`), through `PakSet`, `AssetSource` and
    the loose sources;
  - `crate::world_export_pipeline` (`cli`, `chunk_partitioner`, `topo`, `enfusion_texture_decoder`,
    `export_preparation::aerial_cell_catalog`) and `crate::map_raster_pipeline` (the orthophoto
    stitch, the cartographic render and the water classifier), through `PakVfs`;
  - `crate::enfusion_tooling::cli`, for `enf extract` and `enf dump-entry`.
- Rules: both policies share one parser and one decompressor and differ only through `ReadPolicy`
  (`lookup_and_duplicate_policies_remain_distinct`,
  `raw_deflate_is_world_only_and_metadata_is_preserved`,
  `decompressed_length_checks_follow_the_consumer_policy` and
  `malformed_archives_fail_blueprint_and_are_skipped_by_world` in `tests/policy_parity.rs`); the
  tests build synthetic archives, and the two checks against a real install (`real_pak_census`,
  `real_pak_farmhouse_xob_matches_extract`) are ignored by default.
