# World line-of-sight probe

The body of `cargo xtask map world-los`: it builds the map engine's world occluder over one 512 m
cell of a committed terrain and its eight neighbours, then reports on it, traces segments through
it, or replays an engine-recorded oracle of sight lines against it.

## Contents

```text
tools/map_assets/map_asset_verification/src/world_line_of_sight/
└── gunzip_json.rs  `load_cell`, `load_dem`, `replay` and `run`: the argument loop and every mode
```

## How it works

`tools/map_assets/map_asset_verification/src/world_line_of_sight.rs` holds the public types
(`Dem`, `WorldPair`, `WorldParityFile`, `ReplayReport`) and re-exports the four functions here.
`run` parses the arguments by hand, then `load_cell` reads `objects/prefabs.json.gz`, each of the
nine chunks as `objects/chunks/<cx>_<cy>.bin` or, when no binary exists, `<cx>_<cy>.json.gz`, and
every prefab descriptor and BLAS sidecar under `prefabs/` that the placed prefabs name. The terrain
folder defaults to `assets/terrains/everon/`; `--assets <dir>` points elsewhere.

- `--census`: rows, kinds and proxy rows per loaded chunk.
- `--probe ax,ay,az bx,by,bz`: one segment in the engine frame (x east, y up, z north), with its
  hits and verdict.
- `--bench N`: N random eye-height segments in the cell, timed per segment.
- `--pairs <json>`: replays a `world-parity` oracle file under the block policy that
  `--glass-blocks`, `--foliage-blocks` and `--proxy-only` adjust, counting agreements and the
  phantom and missed blocks by the engine's hit prefab; `--dem` adds the terrain column from the
  elevation model the manifest names; `--dump-misses <jsonl>` writes every disagreement.

`run` exits 0, or 1 on an unknown argument or when `--min-agree F` is set and agreement falls
below it; a missing `--cell` or an unreadable file is an error.

## Boundaries

- Depends on: the world occluder (`world_line_of_sight`), BVH sidecars
  (`spatial_indexes::bounding_volume_hierarchy::sidecar`), chunk parsers (`world_chunks`), prefab
  rows (`prefab_catalog::prefab_rows`) and elevation sampling (`terrain_elevation`); `repository_layout::terrain_dir`.
- Used by: `cargo xtask map world-los`, through `tools/xtask/src/commands/map/mod.rs`; the
  pinned replays in `tools/map_assets/map_asset_verification/src/tests/world_line_of_sight.rs`,
  which read the oracle files in `tools/map_assets/blueprint_compiler/test_fixtures/blueprint/`.
- Rules: the probe reads only committed terrain files and never writes one, except the
  `--dump-misses` file it is given.
