# Forest density fixture

The golden pair of the `TBDD` forest-density tile format: a synthetic set of tree and rock
positions, and the tile those positions must encode to, byte for byte.

## Contents

```text
contracts_v2/fixtures/map/density/
├── density-fixture.bin   the committed `TBDD` tile the fixture's positions must encode to
└── density-fixture.json  the positions, the chunk, the expected file size and the expected corners
```

## Format

- Encoding: `density-fixture.bin` is one binary `TBDD` tile as
  `apps/website/map-engine/src/io/density/` encodes it (16916 bytes at the current cell size), a
  plain git blob rather than LFS. `density-fixture.json` is UTF-8 JSON with `description`,
  `worldSizeM`, `chunk` (`cx`, `cy`), `treePositions` and `rockPositions` (`x`, `y` in world
  metres), `expectedCorners` (sparse `i`, `j`, `tree`, `rock` counts) and `expectedFileBytes`.
- Schema: no JSON Schema; gate S13 of `cargo xtask schema map-object-golden` defines the pair. A
  corner (`i`, `j`) of chunk (`cx`, `cy`) counts the instances in the cell square centred on it at
  the density cell size.
- Adding a file: none is added. After a change to the density cell size, regenerate the tile and the
  expected fields with `cargo run -q -p developer-tools --bin world -- gen-density-fixture`.

## Producers and consumers

- Producers: people write the positions; the `world` binary's `gen-density-fixture` command
  (`gen_density_fixture` in
  `tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/redensify_from_committed.rs`)
  reads them and rewrites `density-fixture.bin`, `expectedCorners` and `expectedFileBytes` in the
  folder `density_fixtures_dir` names (`tools_v2/developer-tools/src/repository_layout.rs`).
- Consumers:
  - gate S13 in `tools_v2/developer-tools/src/map_verification/object_goldens/spatial_invariants.rs`,
    run by `cargo xtask schema map-object-golden` and the `schema-validate` CI task: encoding the
    positions must give exactly `density-fixture.bin`, whose header must match the library
    constants and whose decoded corners must equal `expectedCorners`;
  - the vegetation density tests in
    `tools_v2/developer-tools/src/world_export_pipeline/tests/vegetation_density/tests.rs`, which
    read `density-fixture.bin`.

## Boundaries

- Depends on: the `TBDD` encoder and decoder in `apps/website/map-engine/src/io/density/` and the
  density cell size of the world export pipeline.
- Used by: the map-object golden gate and the developer tools' tests above.
- Rules: the two files agree byte for byte through the encoder (gate S13 of
  `cargo xtask schema map-object-golden`) and change only together, by regeneration.
