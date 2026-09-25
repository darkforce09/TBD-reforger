# Prefab batch entry and helpers

The two submodules of `batch_processing.rs` in `tools_v2/developer-tools/src/blueprint/bvh/`: the
single-building `cargo xtask map bvh-batch --prefab` entry, and the helpers every walk of the game
paks shares, from opening the asset sources to classifying a placed mesh.

## Contents

```text
tools_v2/developer-tools/src/blueprint/bvh/batch_processing/
├── default_extract_dir.rs  asset sources, surface kinds, decoding, cover and kind rules, the writer
└── run_bvh_batch.rs        `run_bvh_batch`: one building's shell sidecar, meshes, instances, scene
```

## How it works

`run_bvh_batch` hands any argument list that holds `--all-prefabs` to the whole-catalogue arm in
`tools_v2/developer-tools/src/blueprint/archive_emission/archive_command.rs`. Otherwise it resolves
`--prefab <Prefabs/…/X.et>`, walks it with the parent's `Walker` from identity, and writes under
`--out` (default `assets_v2/terrains/everon/prefabs/`):

- `buildings/<slug>.bvh`, the shell sidecar built from the building's own collision mesh, with
  `--kind <record>=<opaque|glass|foliage>` overrides bound to that model;
- `blas/<stem>.bvh` for every mesh the instances name;
- `buildings/<slug>.instances.json`, every collision-bearing child placed in the building's frame;
- `buildings/<slug>.scene.json` when `--scene <spec.json>` names hand-placed roots, walked with
  source `scene`.

The slug defaults to the prefab's file stem. Both documents are validated against
`contracts_v2/definitions/building-instances.schema.json` before anything is written, and
`--dry-run` prints the report and writes nothing. `--all-layers` keeps every collider record; by
default only records whose layer preset stops a projectile reach a sidecar.

`default_extract_dir.rs` holds what the walks share. `open_sources` layers the game paks (`--paks`,
else `~/.cache/enfusion-mcp-root/addons`) over a loose extract (`--extract`, else
`~/ReforgerExtract/unpacked` when it exists) and fails when neither exists. `decode_asset` parses a
model's COLL chunk and node table, gives each triangle a surface kind (an override, then its game
material, then its record's layer preset, then opaque), applies the layer policy and emits the
sidecar bytes. `classify_prefab` and `cover_for_prefab` name a placed mesh's kind (door leaf, tree,
furniture and so on) and its cover tier from the prefab path and the mesh's kinds. `write_if_changed`
leaves a file untouched when its bytes already match, so a re-run over unchanged inputs writes
nothing.

## Boundaries

- Depends on: the parent's `Walker`, `AssetCache`, `Asset`, `LayerPolicy` and `SceneSpec`; the pak
  reader in `tools_v2/developer-tools/src/enfusion_pak/`; the model decoders in
  `tools_v2/developer-tools/src/blueprint/mesh_decoding/`; the surface classification in
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/surface_classification.rs`;
  `crate::repository_layout` (`terrain_dir`, `definition_path`); `website_map_engine::spatial::bvh`
  for the sidecar codec and `website_map_engine::world::architecture::compound` for the instance
  records; `jsonschema`.
- Used by: `batch_processing.rs`, which re-exports `run_bvh_batch`, `open_sources`,
  `decode_asset`, `classify_prefab` and `cover_for_prefab`; the prefab library and archive writer in
  `tools_v2/developer-tools/src/blueprint/archive_emission/` (`write_if_changed`, `slug_of`,
  `open_sources`); `xob-inspect` in `tools_v2/developer-tools/src/blueprint/mesh_decoding/`;
  `cargo xtask map bvh-batch`, through `developer_tools::blueprint::run_bvh_batch`.
- Rules: nothing read from the paks is written except the derived sidecars and JSON; the layer
  policy keeps fire records and drops physics shells
  (`layer_policy_keeps_fire_records_and_drops_physics_shells`), and the cover and kind rules are
  pinned (`cover_and_kind_heuristics`), both in
  `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`.
