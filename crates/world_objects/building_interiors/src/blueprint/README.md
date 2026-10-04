# Building blueprints

The floor-by-floor model of one building prefab: its levels with their walls, doors, windows,
stairs and furniture, its height profile, footprint and roof, read from the blueprint JSON or its
binary archive, and the sight-line attribution that names what a ray through the building met.

## Contents

```text
crates/world_objects/building_interiors/src/blueprint/
├── archive.rs            `BuildingBlueprint::from_archived`: the tactical subset of an archived row
├── footprint.rs          the height profile, the overall footprint, and the roof and floor-plate grids
├── level_annotations.rs  per-level annotations: windows and open doors passed, cover and stairs crossed
├── mod.rs                the module tree
├── sight_line.rs         `annotate_sight_line`, the hit and result types, and the naming of a structural hit
├── structure.rs          the blueprint JSON model: levels with walls, doors, windows, stairs, furniture
└── tests/                unit tests for the JSON model, band clipping and hit attribution
```

## How it works

A blueprint is `prefabs/buildings/<slug>.json` in a terrain's asset folder, such as
`assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01.json`: camelCase JSON that serde reads
into `BuildingBlueprint` (`structure.rs`). Its `VerticalProfile` gives the pivot offset,
foundation skirt, eave, ridge and total heights and the roof type; its `OverallFootprint` the plan
polygon, bounding box and area; an optional `RoofGrid` the top surface. Each `BuildingLevel` covers
an `elevationRange` band and holds its footprint polygon, an optional `PlateGrid` and traced
`floorPolygons` (outer ring counter-clockwise, holes clockwise), and its walls, doors, windows,
stairs and furniture. Everything is in the building's local frame: metres, y up, plan points as
`[x, z]`. `RoofGrid` and `PlateGrid` store heights row-major (`ix * nz + iz`, `None` where nothing
covers the cell); `height_at` reads the nearest cell and never interpolates across a gap, and a
grid whose shape fails `is_valid` is skipped by every reader.

`BuildingBlueprint::annotate_sight_line(occl, obs, tgt)` checks a segment against the building's
collision mesh, a `BvhSidecar`:

```text
for each level whose band the segment enters (clip_t_to_band)
    collect_level_annotations -> Window / DoorOpen (concealment 0), Furniture (0.60 low cover,
                                 1.0 full cover), Stairs (the stairs' losConcealment)
sort by t, drop duplicates of one id at one t
first mesh hit (occl.bvh.first_hit) -> attribute_structural_hit names it, concealment 1.0:
    nearest wall within WALL_ATTR_NEAR_M (0.35 m) on its level: Window inside its aperture,
    else Wall
    -> Roof within ROOF_ATTR_TOL_M (0.30 m) of the roof grid -> Stairs -> Furniture -> Solid
walk the events in t order; the first with concealment 1.0 ends the ray (is_clear = false)
```

A level band is half-open, `[min, max)`, and the topmost is closed, so a horizontal ray on a shared
floor-ceiling boundary belongs to exactly one level. An aperture matches within half its width
plus `APERTURE_SLACK_M` (0.05 m). The verdict comes from the mesh: the blueprint names the hit and
adds the annotations, and full-cover furniture is the one annotation that ends a ray without a mesh
hit. Every triangle of the sidecar counts here, whatever its surface kind; the compound walk of
`interior_line_of_sight` (`crates/line_of_sight/interior_line_of_sight/`) tells glass and foliage
apart and produces the
other `LosHitKind` values (`Glass`, `DoorLeaf`, `DoorFrame`, `DoorAperture`, `WindowFrame`,
`Foliage`, `Prop`).

`from_archived` reads a row of `prefabs/building_blueprints.rkyv`
(`world_file_formats::archives::blueprints`) back into a `BuildingBlueprint` with the height profile and
each level's band, footprint, walls, doors, windows, stairs and furniture. The archive carries no
identity strings, overall footprint, roof, floor plates, door state, furniture size or pane count,
so those come back empty or zero.

## Public surface

- `structure`: `BuildingBlueprint` and its level and feature types (`BuildingLevel`,
  `BuildingWall`, `BuildingDoor`, `BuildingWindow`, `BuildingStairs`, `BuildingFurniture`,
  `FloorPolygon`, `BBox2D`), which the blueprint tooling writes and the debug building viewer reads.
- `footprint`: `VerticalProfile`, `OverallFootprint`, `RoofGrid` and `PlateGrid`.
- `sight_line`: `annotate_sight_line`, `attribute_structural_hit` and the result types every
  line-of-sight layer reports in (`LosHit`, `LosHitKind`, `LosResult`, each event and result
  naming its feature by a `BuildingFeatureId`); `clip_t_to_band`.
- `archive`: `BuildingBlueprint::from_archived`.

## Boundaries

- Depends on: `spatial_indexes` (`BvhSidecar` and its first-hit traversal),
  `geometry_primitives::segment_geometry` (the 2D segment and box tests),
  `world_file_formats::archives::blueprints` (the archived rows) and `world_file_formats::ids`
  (the wall, door, window, stairs and furniture identifiers); `serde` for the JSON.
- Used by:
  - `interior_line_of_sight` (`crates/line_of_sight/interior_line_of_sight/`), whose compound
    walk names shell hits with `attribute_structural_hit` and reports in `LosResult`, and
    `world_line_of_sight`, whose world occluder reports in `LosHit` and `LosHitKind`;
  - `crate::section`, whose drawing takes a blueprint's level bands;
  - the debug building viewer and line-of-sight benches in
    `crates/frontend/workspaces/debug_benches/src/`, which fetch the blueprint JSON from
    `/map-assets/everon/prefabs/buildings/`;
  - the blueprint tooling in `tools/map_assets/blueprint_compiler/src/`:
    `cargo xtask map blueprint-from-voxels` writes the blueprint JSON, its `archive` action
    writes the archive, and its parity report compares `annotate_sight_line` with the engine.
- Rules: the Everon farmhouse JSON parses into the model (`parses_farmhouse_blueprint_json` in
  `tests/sight_line_tests.rs`); a horizontal ray on a band boundary belongs to the upper level
  (`clip_t_to_band_horizontal_boundary_belongs_to_upper_level`); a structural hit takes the name
  of the feature that owns it and `Solid` when none does (`blocked_wall_is_named`,
  `roof_attribution_near_surface`, `solid_when_blueprint_silent`); a terminal hit inside an
  aperture is never counted as traversed (`terminal_hit_inside_aperture_is_not_traversed`); an
  absent roof, plate or floor polygon list is left out of the JSON
  (`blueprint_without_roof_is_unchanged`, `plate_grid_and_floor_polygons_round_trip`).

## Related documentation

- [Building blueprint schema](/contracts/definitions/building-blueprint.schema.json) — the
  blueprint JSON.
