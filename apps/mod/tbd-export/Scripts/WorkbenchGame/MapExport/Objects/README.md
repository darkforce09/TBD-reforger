# World object export

The placed-object layers of the map export, read from the world open in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench): the full world-object export the developer
tools build the map's object data from, a classified object export, building blueprints,
infrastructure, and a line-of-sight reference sampler.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/
├── Buildings/                   building blueprints, their Net API handler, placed buildings by type
├── Infrastructure/              fences, bridges and piers, runways and helipads, and the power grid
├── Props/                       a props exporter that resolves its path and writes nothing
├── TBD_MapExportObjects.c       every entity sorted into buildings, props, trees and rocks
├── TBD_ObjectsExportPlugin.c    the standalone plugin that runs the classified object export
├── TBD_WorldFullExportPlugin.c  `TBD_WorldFullExportPlugin`: every entity to one JSON-lines file
└── TBD_WorldTraceParity.c       seeded engine sight traces inside one 512 m cell
```

## How it works

The two world exporters here share one sweep. The world is cut into square cells, and each cell is
queried with `BaseWorld.QueryEntitiesByAABB` from −1,000 m to 2,000 m in height; an entity counts
only in the cell that holds its origin, `clamp(floor(coord / cell), 0, cells − 1)` on x and z,
and an entity outside the world is skipped. Each kept entity gives a row with its prefab resource
name, class name, position, `GetAngles` as `pitchDeg`, `headingDeg` and `rollDeg`, uniform
`scale` (1.0 when 0.001 or less) and `halfExtentsM` from its world bounds.

```text
TBD_WorldFullExportPlugin ──▶ $profile:TBD_WorldExport_full.jsonl + ..._full_meta.json
        │
        ▼  world copy-export-profile --full
assets_v2/scratch/<terrain>/export/raw-entities.jsonl
        │
        ▼  cargo xtask map export-terrain <terrain> --phase <P>
world build-objects, world build-roads ──▶ assets_v2/terrains/<terrain>/objects/, roads/
```

### Full world-object export

`TBD_WorldFullExportPlugin.Run` reads the terrain bounds, deletes any old meta file, and writes
every entity in fixed 512 m cells to `$profile:TBD_WorldExport_full.jsonl`, in the profile root
rather than under `TBD_Export/`, one JSON object per line: `resourceName`, `className`, `x`, `y`,
`z`, `headingDeg`, `pitchDeg`, `rollDeg`, `scale`, `halfExtentsM`. When the file is closed it
writes `$profile:TBD_WorldExport_full_meta.json` last, as the completion sentinel:
`exportVersion` 2, `worldSizeM`, `cellSizeM`, `cells`, `aabbHitCount`, `keptCount`, `withPrefab`,
`withScale`, `outOfBounds`, `elapsedMs`, and the angle, scale and partition rules as text. A failed
write deletes the partial file and writes no meta. The cell rule matches `cell_of` in
`tools_v2/developer-tools/src/world_export_pipeline/polygon_geometry.rs`.

`world copy-export-profile --full`
(`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_profile.rs`) refuses
to stage the file without the meta, or when `keptCount` differs from the file's line count, and
stages it as `raw-entities.jsonl` for `cargo xtask map export-terrain`. The plugin's
`[WorkbenchPluginAttribute]` (menu "Export TBD World Objects (full)") is commented out and no Net
API action calls it, so no committed path runs the export, although the operator steps that
`cargo xtask map export-terrain` prints name that menu entry.

### Classified object export

`TBD_MapExportObjects.Export` sweeps in cells of `m_fObjectChunkSizeM` (512 m by default) and sorts
each row with `ClassifyEntity`, by substrings of the class and prefab names, into a building
(subtype `residential`, `military`, `commercial`, `industrial`, `civic`, `sheds_garages` or
`generic`), a tree (`conifer`, `deciduous`, `generic_tree`), a rock (`cliff`, `boulder`) or a prop
(`container`, `barrier`, `crate`, `street_furniture`, `clutter`); the row gains a `subtype` field.
It writes under `$profile:TBD_Export/<map>/`:

- `buildings/all_buildings.jsonl`, one array file per building subtype except `generic`, and
  `buildings/buildings_meta.json` with the counts by subtype;
- `props/props.jsonl` and `props/props_meta.json`;
- `vegetation/trees.jsonl`, `vegetation/rocks.jsonl` and `vegetation/vegetation_meta.json`. The
  vegetation coordinator in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/`
  writes a `vegetation_meta.json` of its own to the same path, and `TBD_MapExportPlugin` runs the
  vegetation layers first, so this file replaces it.

`TBD_ObjectsExportPlugin` builds its own config and context and runs the classified export;
`Configure` opens the config dialog. `TBD_MapExportPlugin` in
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs it behind `m_bExportObjects`
and the infrastructure exporters behind their own switches; it runs neither the full export, the
building blueprints nor the props placeholder. Every `[WorkbenchPluginAttribute]` here and in the
child folders is commented out.

### World sight parity

`TBD_WorldTraceParity.Execute(cx, cy, samples, seed)` is the `world-parity` action of the
`EMCP_WB_TbdBlueprint` handler in `Buildings/`. It seeds the random generator, draws `samples`
(2,000 when not positive) observer and target pairs in one 512 m cell in four strata taken in turn
(eye height 10–300 m apart, both ends inside a random entity's padded bounds, eye height 300–500 m
apart, 0.4–8 m above ground 10–200 m apart), and traces each pair on the `Projectile` layer twice,
entities only and entities with terrain. It writes
`$profile:TBD_Export/<map>/prefabs/debug/world_parity_<cx>_<cy>.json` with the settings, the
entity pool size, each pair as `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, "hitSlug"]` and
the two clear counts, where `hitSlug` names the prefab the entity trace hit.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`;
  `TBD_BuildingArchitectExtractor.DerivePrefabSlug` in `Buildings/`; Workbench's `WorldEditor`
  module and `WorldEditorAPI`; the engine's `BaseWorld.QueryEntitiesByAABB` and `TraceMove`.
- Used by:
  - `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`;
  - `world copy-export-profile --full`, which stages the full export, and through it
    `cargo xtask map export-terrain` and `map export-locations`
    (`tools_v2/developer-tools/src/map_raster_pipeline/map_labels/importance_by_name.rs`);
  - `cargo xtask map world-los`, which replays the world parity file. No committed tool reads the
    classified export.
- Rules: both world sweeps here keep an entity only in the cell of its origin, with the same clamp
  as the developer tools' `cell_of`; the full export writes its meta last and only after a complete
  file, so a crashed run never stages; lines added stay ASCII. `cargo xtask mod compile` compiles
  only the framework addon, so these scripts compile only when Workbench loads `tbd-export`.

## Related documentation

- [World export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) — staging
  the full export and building the object and road data from it.
- [Everon object data](/assets_v2/terrains/everon/objects/README.md) — the committed chunks and
  catalogue the full export feeds.
- [Map commands](/tools_v2/xtask/src/commands/map/README.md) — `cargo xtask map export-terrain` and
  the parity commands.
- [Terrain export runbook](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md) —
  the full export through `copy-export-profile`, `export-terrain` and `verify-phase`.
