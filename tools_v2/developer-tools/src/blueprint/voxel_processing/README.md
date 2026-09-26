# Voxel dump processing

The voxel dump side of the blueprint compiler: the in-memory model of a `tbd-voxel-dump/1` file,
its strict reader, the tunables every interpretation stage reads, the generator that writes a dump
from a game model's triangles, and the analytic buildings the tests march. A voxel dump is the
record of axis-aligned ray marches through one building that the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) dump action or `voxels-from-mesh` writes.

## Contents

```text
tools_v2/developer-tools/src/blueprint/voxel_processing/
├── analysis_parameters.rs  `Params`: every interpretation tunable, overridable from `--params`
├── dump_parser.rs          the strict dump reader: meta, scanlines, furniture records, end line
├── mesh_voxelization/      the `voxels-from-mesh` entry, the triangle march and the dump writer
├── mesh_voxelization.rs    `TriMesh` and `AxesRemap`: a game model's triangles ready to march
├── synthetic_fixtures.rs   analytic test buildings: boxes, a door, gables, a mezzanine
└── voxel_types.rs          `VoxelDump`, `DumpMeta` and the grids and scans later stages share
```

## How it works

Every file here is a module of the blueprint root, declared in
`tools_v2/developer-tools/src/blueprint/mod.rs` by `#[path]` under a short name: `params`,
`parse`, `mesh`, `synth` and `types`.

```text
Workbench dump action ──▶ <slug>_voxels.jsonl[.gz] ◀── voxels-from-mesh (mesh_voxelization/)
                                   │
                          dump_parser::parse_dump ──▶ VoxelDump (voxel_types)
                                   │
            blueprint-from-voxels: slabs, bands, walls, plates, roof (architectural_analysis)
                                   │ reads Params (analysis_parameters)
                                   ▼
                    BuildingBlueprint JSON (archive_emission)
```

A dump file is one meta object (version `tbd-voxel-dump/1`, slug, resource, the padded origin, the
0.1 m cell, dimensions, span, the unpadded bounds, the root yaw and the counts of excluded doors,
glass and furniture), then one line per scanline (`["x+", j, k, [hits…]]`), `{"furn": …}` records,
and a closing `{"end": {"lines": n, "ms": t}}`. Coordinates are metres from the meta origin;
the emitter adds the origin back to return to the building's local frame. `parse_dump` reads plain
or gzipped files and fails on the first broken convention: a version other than
`tbd-voxel-dump/1`, a first line that is not the meta object, an unknown, empty or duplicate
scanline, a non-finite hit, hits out of march order (`+` runs ascend, `-` runs descend), data after
the end line, a missing end line, or an end line whose count disagrees with the lines read. A
scanline the dumper cut at its 48-hit cap carries a fifth element; the parser counts it as
truncated, and the interpreter warns and keeps it.

`Params` holds 47 tunables for face pairing, slab detection, wall clustering, floor plates, the
roof grid and the attic band. It deserialises with `deny_unknown_fields`, so a `--params` file
overrides some keys, keeps the defaults for the rest and refuses an unknown key.

## Public surface

- `developer_tools::blueprint::run_voxels_from_mesh`, re-exported by the blueprint root and run by
  `cargo xtask map voxels-from-mesh`.
- Everything else stays inside the blueprint compiler: the analysis, assembly and batch modules
  read `types`, `parse` and `params`.

## Boundaries

- Depends on: the march skeleton and the model reader of the blueprint root
  (`tools_v2/developer-tools/src/blueprint/architectural_analysis/contour_tracing.rs`,
  `tools_v2/developer-tools/src/blueprint/mesh_decoding/`); `serde`, `serde_json` and `flate2`.
- Used by: the blueprint root's `run` and `interpret_one`; the modules in
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/`,
  `tools_v2/developer-tools/src/blueprint/archive_emission/blueprint_assembly.rs` and
  `tools_v2/developer-tools/src/blueprint/bvh/instance_pairs.rs`; the blueprint tests in `tools_v2/developer-tools/src/blueprint/tests/`,
  which march the synthetic buildings and parse
  `tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_voxels.jsonl.gz`.
- Rules: the wire format is the one `TBD_BuildingVoxelDump.c` writes
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingVoxelDump.c`),
  and the parser refuses rather than repairs a dump that breaks it
  (`missing_end_line_is_truncation`, `wrong_line_count_fails` and `march_order_violation_fails` in
  `tools_v2/developer-tools/src/blueprint/tests/parse/tests.rs`); an unknown `--params` key fails
  (`unknown_key_rejected` in `tools_v2/developer-tools/src/blueprint/tests/params/tests.rs`); the
  synthetic buildings compile only into test builds.
