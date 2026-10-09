# Voxel dump processing

The voxel dump side of the blueprint compiler: the in-memory model of a `tbd-voxel-dump/1` file,
its strict reader, the tunables every interpretation stage reads, the generator that writes a dump
from a game model's triangles, and the analytic buildings the tests march. A voxel dump is the
record of axis-aligned ray marches through one building that the
[Workbench](/documentation/glossary/n_to_z.md#workbench) dump action or `voxels-from-mesh` writes.

## Contents

```text
tools/map_assets/blueprint_compiler/src/voxel_processing/
├── analysis_parameters.rs  `AnalysisParameters`: every interpretation tunable, overridable from `--params`
├── dump_parser.rs          the strict dump reader: meta, scanlines, furniture records, end line
├── mesh_voxelization/      the `voxels-from-mesh` entry, the triangle march and the dump writer
├── mesh_voxelization.rs    `TriangleMesh` and `AxesRemap`: a game model's triangles ready to march
├── synthetic_fixtures.rs   analytic test buildings: boxes, a door, gables, a mezzanine
├── tests/                  the unit tests of the reader, the tunables and the mesh voxelizer
├── voxel_dump_lattice.rs   the shared march lattice: 0.1 m cell, padding, march order, `round_to_two_decimals`
└── voxel_types.rs          `VoxelDump`, `DumpMetadata` and the grids and scans later stages share
```

## How it works

Every file here is a module of `voxel_processing`, declared in `tools/map_assets/blueprint_compiler/src/voxel_processing.rs`;
the unit tests are in `tests/`, one file per module.

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

`voxel_dump_lattice.rs` holds the wire conventions of the Workbench sensor
(`TBD_BuildingTraceScanner.c`): the scan box is the bounds padded 0.6 m (1.2 m above), the cell is
0.1 m, coordinates are rounded to two decimals, and empty scanlines are omitted. The analytic test
buildings and `voxels-from-mesh` both march through `generate_dump`, so the two generators differ
only in their intersection maths; the architectural stages read its `round_to_two_decimals` rounding.

`AnalysisParameters` holds 47 tunables for face pairing, slab detection, wall clustering, floor plates, the
roof grid and the attic band. It deserialises with `deny_unknown_fields`, so a `--params` file
overrides some keys, keeps the defaults for the rest and refuses an unknown key.

## Public surface

- `blueprint_compiler::run_mesh_voxelization`, re-exported by the blueprint root and run by
  `cargo xtask map voxels-from-mesh`.
- Everything else stays inside the blueprint compiler: the analysis, assembly and batch modules
  read `voxel_types`, `dump_parser` and `analysis_parameters`.

## Boundaries

- Depends on: the march skeleton and the model reader of the blueprint root
  (`tools/map_assets/blueprint_compiler/src/voxel_processing/voxel_dump_lattice.rs`,
  `tools/map_assets/blueprint_compiler/src/mesh_decoding/`); `serde`, `serde_json` and `flate2`.
- Used by: the blueprint root's `run` and `interpret_one`; the modules in
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/`,
  `tools/map_assets/blueprint_compiler/src/archive_emission/blueprint_assembly.rs` and
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/collision_face_pairing.rs`; the blueprint tests in `tools/map_assets/blueprint_compiler/src/`,
  which march the synthetic buildings and parse
  `tools/map_assets/blueprint_compiler/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_voxels.jsonl.gz`.
- Rules: the wire format is the one `TBD_BuildingVoxelDump.c` writes
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingVoxelDump.c`),
  and the parser refuses rather than repairs a dump that breaks it
  (`missing_end_line_is_truncation`, `wrong_line_count_fails` and `march_order_violation_fails` in
  `tools/map_assets/blueprint_compiler/src/voxel_processing/tests/dump_parser_tests.rs`); an unknown `--params` key fails
  (`unknown_key_rejected` in `tools/map_assets/blueprint_compiler/src/voxel_processing/tests/analysis_parameters_tests.rs`); the
  synthetic buildings compile only into test builds.
