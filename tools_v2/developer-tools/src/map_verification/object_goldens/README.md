# Map-object golden gate

The semantic gate over the map-object golden fixtures: sub-gates S2 to S9 and S11 to S15 check
that the committed samples in `contracts_v2/fixtures/map/` agree with the enums, with the world
export's own geometry, density and forest code, and with the chunk binary its emitter writes.

## Contents

```text
tools_v2/developer-tools/src/map_verification/object_goldens/
├── map_object_golden.rs   `map_object_golden`: the fixtures, S2 to S9, S15 and the report
├── read_json.rs           the fixture reader, row helpers and the S15 chunk-binary comparison
└── spatial_invariants.rs  S11 to S14: chunk sample, anchor, density and region fixtures
```

## How it works

`tools_v2/developer-tools/src/map_verification/object_goldens.rs` declares the three files, holds
the `Gate` record (an id, a label, the errors) and re-exports `map_object_golden`. That function
reads `map-object-enums.schema.json`, then the samples under `contracts_v2/fixtures/map/` (the
prefab, instance, region, road, resolved and chunk samples, `phased/P1-anchor-fixture.json`,
`density/density-fixture.json` with its `.bin`, `regions-derivation-fixture.json`, and the three
catalogue bundles), and pushes one `Gate` per sub-gate:

| Gates | What holds |
|---|---|
| S2 to S7 | every row has a resolvable kind and class; one prefab example per instance kind; valid road classes; unique prefab ids and names; instance prefab ids resolve; each prefab carries its AI, cover and height fields |
| S8, S9 | the resolved sample validates `map-object-resolved.schema.json`; every closed enum value has an example |
| S11 | the chunk sample's rows: tuple width, trailers, bounds, order and zoom coverage |
| S12 | the anchor fixture partitions and passes `check_anchors`, as `world verify-phase` runs it |
| S13 | `accumulate_corners` and `slice_chunk_corners` reproduce the density fixture byte for byte |
| S14 | `derive_forest_regions` reproduces the region fixture's rings and totals |
| S15 | `write_chunk_bin` re-emits `map-object-chunk-sample.bin` byte for byte, and the binary decodes to the JSON rows |

The report prints a `PASS` or `FAIL` line per gate and returns 0 when no gate has an error, 1
otherwise. A missing S15 binary is a failure, never a skipped gate.

## Boundaries

- Depends on: the world export pipeline's `binary_emit`, `forest_contours`, `polygon_geometry` and
  `vegetation_density` (`tools_v2/developer-tools/src/world_export_pipeline/`);
  `website-map-engine`'s chunk parsers (`streaming::loaders::chunk`, `chunk_bin`), prefab maps
  (`world::environment::buildings::prefab`) and container constants (`io::containers::header`,
  `io::pod::instance`); the fixtures and schemas under `contracts_v2/`.
- Used by: `cargo xtask schema map-object-golden`, through
  `tools_v2/xtask/src/verifications/map_assets/mod.rs`, and the CI task `schema-validate`.
- Rules: the gate computes with the exporter's and the loader's own functions, never a copy, so a
  golden that passes here is what the pipeline writes and the map reads; after a density cell-size
  change, `world gen-density-fixture` rewrites the S13 fixture.

## Related documentation

- [Map fixtures](/contracts_v2/fixtures/map/README.md) — the golden samples this gate reads.
