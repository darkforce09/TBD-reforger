# Export artifact validation

The bodies of `world validate-exports` and `world spike-ops-log`: the first checks every
registered terrain's committed world-export artifacts against their schemas and against each
other, the second checks the operations log of a subregion spike export.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_validation/
├── artifact_integrity.rs  `validate_export_artifacts`: registry, catalogue, chunks, roads, density, E2
└── operations_log.rs      `verify_spike_ops_log`: the spike's operations log and its K gates
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/export_preparation.rs` declares both files
with `#[path]` and re-exports their functions; each prints one `PASS` or `FAIL` line per check and
returns 0 when all held, 1 otherwise.

`validate_export_artifacts` validates `assets_v2/terrains/terrain-registry.json` against its
schema, then walks the terrain registry's entries. A terrain whose manifest is missing, or whose
manifest has no `objects.prefabsPath`, is reported and skipped whatever its `status` says. For the
others it checks, with the schemas `SchemaSet` loads from `contracts_v2/definitions/`:

- the prefab rows of `objects/prefabs.json.gz`;
- every cell that `objects/chunks/manifest.json` lists, read as gzip JSON: its row count against
  the index and each row's partition, then the manifest's `instanceCount` and `prefabCount`;
- `objects/roads.json.gz` against its schema, with at least one segment;
- when the manifest names `densityPath`: the tile count, `densityCellM`, each tile's `TBDD` header
  and its tree channel against a recount from the chunks;
- when it names `regionsPath`: the forest region rows.

It then runs `cargo run -q -p xtask -- schema type-inventory` as a child process, and the
cross-terrain E2 checks: E2a, at least two terrains in the registry; E2b, `cargo xtask map
export-terrain <terrain> --phase P1_buildings` exits 2 for a terrain with no staged export; E2c, no
literal terrain id in seven pipeline source files, a line marked `E2c-allow` excepted.

`verify_spike_ops_log` reads `.ai/artifacts/map_export_<terrain>.json` and
`assets_v2/scratch/<terrain>/spike/raw-entities.jsonl`: the log needs its required keys, the
`spike-subregion-export` slice, a finite four-number `subregionBBoxM` and a lowercase `pass` or
`fail` for each of K1, K1b and K2 to K7, and every gate it marks `pass` must hold against the
sampled rows and the files the log names.

## Boundaries

- Depends on: `SchemaSet` and `gunzip_json` from
  `tools_v2/developer-tools/src/world_export_pipeline/mathematical_verification.rs`; the sibling
  `vegetation_density` and `polygon_geometry` modules; `website-map-engine`'s
  `io::density::tbdd` decoder; `crate::repository_layout`; `cargo`, for the two child runs.
- Used by: `world validate-exports` and `world spike-ops-log`
  (`tools_v2/developer-tools/src/world_export_pipeline/cli.rs`); nothing runs either in CI.
- Rules: both only read; the registry `status` gates nothing, because a terrain is checked exactly
  when its manifest and its `objects` block exist.
