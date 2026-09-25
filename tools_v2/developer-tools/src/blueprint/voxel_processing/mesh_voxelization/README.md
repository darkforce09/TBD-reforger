# Mesh voxel dump generator

The two submodules of `mesh_voxelization.rs` in
`tools_v2/developer-tools/src/blueprint/voxel_processing/`: the `cargo xtask map voxels-from-mesh`
entry and the ray-march that turns a game model's triangles into the same voxel dump the
[Workbench](/documentation_v2/glossary.md#workbench) dump action writes, so the offline blueprint
interpreter runs on real model geometry without the engine.

## Contents

```text
tools_v2/developer-tools/src/blueprint/voxel_processing/mesh_voxelization/
├── run_voxels_from_mesh.rs  the `voxels-from-mesh` entry: arguments, frame and winding checks, write
└── sub.rs                   vector helpers, per-line hits, `generate`, `write_dump`, the stats print
```

## How it works

`run_voxels_from_mesh` reads the `.xob` given by `--mesh` and picks the geometry: `--geometry auto`
(the default) takes the fire-collision (COLL) chunk when the model has one and the visual LODs
otherwise; `coll` and `visual` force one, and `--coll-record` and `--lod` narrow it. The decoded
mesh's statistics always print, and `--stats` stops there with exit 0. The parent's
`TriMesh::from_xob` drops `--exclude-material` triangles, applies `--axes` (such as `x,y,-z`) and
`--flip-winding`, and orients each face by the packed vertex normals. With
`--reference <dump.jsonl[.gz]>` the command compares the mesh bounds with a Workbench dump's box,
and it always prints the fraction of first hits that land on back faces, warning when most do.

`generate` pads the model's bounds by the sensor's margins, bins the triangles per axis, and runs
the shared march skeleton with `line_hits` as the intersection. Hits closer than 0.02 m merge
(`min_sep`) where the engine sensor re-casts, and no 48-hit cap applies. `write_dump` writes the
meta line, the `x±`, `z±` and `y∓` scanlines in sorted key order and the end line, gzipped when the
path ends in `.gz`. The output is `<slug>_voxels.jsonl.gz` under `--out`, by default
`target/mesh-dumps/` at the checkout root; the run reads the file back through the strict dump
parser before it reports `OK`.

## Boundaries

- Depends on: the parent's `TriMesh`, `AxesRemap` and `AxisBins`; the model reader in
  `tools_v2/developer-tools/src/blueprint/mesh_decoding/`; the march skeleton
  (`tools_v2/developer-tools/src/blueprint/architectural_analysis/contour_tracing.rs`: the lattice,
  the padding and `generate_dump`); the dump parser
  (`tools_v2/developer-tools/src/blueprint/voxel_processing/dump_parser.rs`) for the self-check;
  `crate::repository_paths::find_repo_root`; `flate2`.
- Used by: `mesh_voxelization.rs`, which re-exports `run_voxels_from_mesh`, `generate` and
  `write_dump`; `cargo xtask map voxels-from-mesh`, through
  `developer_tools::blueprint::run_voxels_from_mesh`; the tests in
  `tools_v2/developer-tools/src/blueprint/tests/mesh/tests.rs`.
- Rules: a written dump passes the strict parser
  (`written_dump_round_trips_through_strict_parser`); a cube's dump equals the analytic box's
  (`cube_matches_analytic_box`); a one-sided sheet registers in one march direction only
  (`open_sheet_is_one_sided`), as engine collision does. All three tests are in that file.
