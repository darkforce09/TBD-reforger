# World export phase gates

The bodies of `world verify-phase` and `world phase-gate`: the mathematical gate that proves a
terrain's committed object artifacts are what the staged export and the pipeline produce for an
import phase, and the registry check that stops an export from running ahead of its approved
phase.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/mathematical_verification/
├── artifact_integrity.rs  G1 to G3, G5 and G12: schemas, prefab ids, chunk index, orphans
├── gunzip_json.rs         `gunzip_json`: reads one gzip JSON file
├── phase_validation.rs    `phase_gate`: refuses a phase above the registry's `importPhaseMax`
├── run_p2_gates.rs        the tree-phase gates: PH-P2-1 to PH-P2-5, D1, D2, F1, F2 and F6
├── tempdir.rs             E6's scratch folders and file lists, and the gates P1-1 to P1-6
└── verify_phase.rs        `verify_phase`: the run, G6 to G11, roads, fences, size, E6
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/mathematical_verification.rs` holds
`SchemaSet`, which loads the nine map-object and terrain-registry schemas from
`contracts_v2/definitions/` into one registry, and the `Gates` list; it declares the six files.

`verify_phase` needs `assets_v2/scratch/<terrain>/export/raw-entities.jsonl` and exits 2 without
it. It reads the committed `objects/` artifacts, the terrain manifest and the terrain registry,
streams the staged export once through the classifier, then adds one gate per check:

| Gates | Runs for | What holds |
|---|---|---|
| G1 to G3, G5 to G12 | every phase | schemas, resolved objects, prefab-id bijection, chunk index against files, partition, count identities, bounds, cover enum, positive sizes, raw-to-catalogue parity for the phase, no orphan prefabs |
| P1-1 to P1-4, P1-6 | `P1_buildings` | building prefabs present, hard cover, footprints, 32 anchors within 2 m, building classes |
| PH-P2-1 to PH-P2-5, D1, D2, F1, F2, F6 | `P2_trees` | tree prefabs and counts, density tiles complete and byte-identical to a recount, forest regions valid, conserved and reproducible |
| P5-1 | `P5_props` | fence prefabs and instances exist |
| R-P1, SIZE | every phase | roads present; gzip chunks under 40 MB in total |
| E6 (with G4 and I6) | every phase | two scratch builds at the manifest's `importPhaseMax` are byte-identical to each other and to the committed files |

E6 runs `build_world_objects_opt` and `build_roads_from_topo_opt` into two temporary folders, so
it needs the game's archives for the road network's `.topo` file. The report prints a `PASS` or
`FAIL` line per gate with up to eight errors each, and exits 0 or 1.

`phase_gate` accepts the ten phases `P1_buildings` to `P10_full` and exits 1 for a terrain
missing from the registry, an unknown phase, or one past the entry's `importPhaseMax`.

## Boundaries

- Depends on: the sibling modules `chunk_partitioner` (the builders E6 reruns, `phase_kinds`,
  `gunzip`), `classify`, `forest_contours`, `polygon_geometry`, `vegetation_density` and
  `json_number_formatting`; `website-map-engine`'s `io::density::tbdd` decoder;
  `crate::repository_layout`; `jsonschema`.
- Used by: `world verify-phase` and `world phase-gate`
  (`tools_v2/developer-tools/src/world_export_pipeline/cli.rs`); `cargo xtask map export-terrain`
  runs `phase-gate` first and prints `verify-phase` as the next step; `world validate-exports`
  borrows `SchemaSet` and `gunzip_json`.
- Rules: a terrain's `importPhaseMax` in `assets_v2/terrains/terrain-registry.json` advances only
  after `world verify-phase` passes for that phase; the gate computes with the pipeline's own
  functions, and the anchor check (`check_anchors`) is the one the golden fixture pins
  (`cargo xtask schema map-object-golden`, S12).
