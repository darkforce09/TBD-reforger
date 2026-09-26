# Building blueprint export

Turns the buildings of the world open in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) into
building blueprints (floors, walls, doors, windows, stairs, furniture and roof heights), measures
line-of-sight reference pairs against the engine, and lists every placed building by type. A Net
API handler lets the developer tools drive the blueprint steps from outside Workbench.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/
├── EMCP_WB_TbdBlueprint.c            the Net API handler that runs one blueprint action per request
├── TBD_BlueprintReconPlugin.c        dumps one building's child tree and components
├── TBD_BuildingArchitectExtractor.c  the `TBD_BuildingBlueprint` model and a bounds-based blueprint
├── TBD_BuildingsExportPlugin.c       the standalone plugin that runs the building instance export
├── TBD_BuildingTraceExtract.c        measured blueprints, probes and parity pairs
├── TBD_BuildingTraceScanner.c        `TBD_BuildingTraceScanner`: raycasts one building's collision
├── TBD_BuildingVoxelDump.c           `TBD_BuildingVoxelDump`: raw entry faces on a 0.1 m lattice
└── TBD_MapExportBuildings.c          `TBD_MapExportBuildings`: every placed building, by type
```

## How it works

### The blueprint actions

`EMCP_WB_TbdBlueprint` is a `NetApiHandler`: Workbench runs it when a Net API request names it as
`APIFunc`. `cargo xtask mcp wbcall EMCP_WB_TbdBlueprint '<json>'` sends that request to
`ENFUSION_WORKBENCH_HOST` and `ENFUSION_WORKBENCH_PORT` (127.0.0.1:5775 by default). The request
holds `action`, `filter` (a prefab-name substring, `FarmHouse_E_1L01` when empty), `maxEntities`
(512 by default), `cx`, `cy`, `seed`, and the probe endpoints `ax` to `bz`. Each action works on the
first entity, cell by cell across the world, whose prefab name contains the filter, and writes
under `$profile:TBD_Export/<map>/prefabs/`:

| `action` | Runs | Writes |
|---|---|---|
| `recon` | `TBD_BlueprintRecon.Execute` | `debug/<slug>_children.json`: the root's transform, bounds and components, then every child to full depth with its name, class, prefab, angles, local and world position, bounds and components (at most `maxEntities` children) |
| `extract` | `TBD_BuildingTraceExtract.Execute` | `buildings/<slug>.json`: a blueprint measured by raycasts |
| `parity` | `TBD_BuildingTraceExtract.ExecuteParity` | `debug/<slug>_parity.json`: `maxEntities` random local-frame observer and target pairs, each with the engine's clear or blocked verdict |
| `probe` | `TBD_BuildingTraceExtract.ExecuteProbe` | nothing; returns one segment's trace results under several flag and mask variants |
| `dump` | `TBD_BuildingVoxelDump.Execute` | `dumps/<slug>_voxels.jsonl`: the raw lattice entry faces |
| `world-parity` | `TBD_WorldTraceParity.Execute` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/` | `debug/world_parity_<cx>_<cy>.json` |

The slug is the prefab file name without its extension. The response carries `status` (`ok` when
the action's summary starts with `OK`, else `error`), the `action` and the summary as `message`; an
unknown action answers `error` with the list of actions.

### Measured blueprints

`TBD_BuildingTraceScanner` casts rays against the building's collision on the `Projectile` physics
layer, the surface that stops gunfire and sight, in the building's local frame so a rotated
instance scans the same. Its trace filter accepts only the building root and the children not
excluded. Before scanning, `TBD_BuildingTraceExtract.ExcludeDressing` excludes door leaves (children
with a `DoorComponent`), destructible panes and boards (`SCR_DestructionMultiPhaseComponent`) and
furniture (children with a prefab of their own). The extract then:

1. marches a vertical 0.25 m grid for the roof (eave, ridge, chimney) and the floor slabs, which
   set the level bands;
2. marches each band horizontally in 0.10 m rows into an occupancy grid, splits it into wall
   rectangles with measured thickness, and traces the level's outline;
3. scans across every wall for openings, a door where the sill is under 0.3 m and a window above;
4. records the excluded furniture as cover, by its bounds and the level it stands on.

What the traces cannot know, such as a door's hinge side, is written as `unknown` or empty. The
parity action excludes only the destructible panes, since closed doors block sight and glass does
not.

`TBD_BuildingVoxelDump` marches the full 0.1 m lattice in all six axis directions with the same
exclusions and streams the raw entry faces, with no interpretation, as JSON lines: a meta line
first, then one `["x+", j, k, [a, b, …]]` line per non-empty scanline, one `{"furn": …}` line per
excluded furniture entity, and a closing `{"end": …}` line whose absence marks a truncated dump.

### Placed buildings

`TBD_MapExportBuildings.Export` sweeps the world in cells of `m_fObjectChunkSizeM`, keeps each
entity whose origin lies in the cell and which `TBD_MapExportObjects.ClassifyEntity` calls a
building, and writes under `$profile:TBD_Export/<map>/objects/buildings/`: `all_buildings.jsonl`
(one instance per line: name, prefab slug, resource name, subtype, position, angles, half extents),
one array per subtype (`residential.json`, `military.json`, `commercial.json`, `industrial.json`,
`civic.json`, `sheds_garages.json`) and `buildings_meta.json`. For the first instance of each
prefab outside the `generic` subtype it also writes `prefabs/buildings/<slug>.json` from
`TBD_BuildingArchitectExtractor.ExtractBlueprint`, which reads the model's bounds and child
entities and lays out levels from a fixed farmhouse template or a generic one, not from traces.
Both blueprint writers share `TBD_BuildingBlueprint.ToJson` and the same file path, so the later
run replaces the earlier file.

### Menu plugins

`TBD_BuildingsExportPlugin` builds its own config and context and runs the placed-building export;
`TBD_BlueprintReconPlugin`, `TBD_BuildingTraceExtractPlugin` and `TBD_BuildingVoxelDumpPlugin` run
one action on a filter attribute. Every `[WorkbenchPluginAttribute]` in the folder is commented out,
so Workbench lists none of them in a menu; the Net API handler is the working entry point.
`TBD_MapExportPlugin` does not run anything here.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`;
  `TBD_MapExportObjects.ClassifyEntity` and `TBD_WorldTraceParity` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/`; the engine's `NetApiHandler`,
  `JsonApiStruct`, `BaseWorld.TraceMove` and `QueryEntitiesByAABB`.
- Used by:
  - `cargo xtask mcp wbcall` (`tools_v2/xtask/src/commands/mcp/netapi.rs`), which calls the
    handler over the Net API;
  - `cargo xtask map ingest-blueprints` (`tools_v2/developer-tools/src/blueprint/ingest.rs`), which
    copies `prefabs/buildings/*.json` from the profile into
    `assets_v2/terrains/everon/prefabs/buildings/`, validated against the `BuildingBlueprint`
    contract;
  - `cargo xtask map blueprint-from-voxels`, which reads `prefabs/dumps/<slug>_voxels.jsonl`;
  - `cargo xtask map parity-report`, `map bvh-parity` and `map world-los`, which replay the parity
    pairs, and `cargo xtask map instances-verify`, which checks instances against a recon dump.
- Rules: the handler stays in this addon, out of any `EnfusionMCP/` folder, which the MCP's
  `wb_cleanup` deletes; a blueprint number comes from a trace against the building's own collision,
  never a constant, in the measured path; the dump stays uninterpreted, so extraction rules change
  in `tools_v2/developer-tools/src/blueprint/`, not here. Lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon, so these scripts compile only when
  Workbench loads `tbd-export`, and a changed handler answers only after Workbench recompiles it.

## Related documentation

- [Building blueprints](/assets_v2/terrains/everon/prefabs/buildings/README.md) — the committed
  blueprints and sidecars, and their consumers.
- [Blueprint compiler](/tools_v2/developer-tools/src/blueprint/README.md) — the offline
  interpretation of the dumps and the parity tools.
- [Map commands](/tools_v2/xtask/src/commands/map/README.md) — the `cargo xtask map` blueprint and
  parity commands.
- [MCP commands](/tools_v2/xtask/src/commands/mcp/README.md) — `cargo xtask mcp wbcall`.
- [Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — the Net API that
  `cargo xtask mcp wbcall` reaches.
