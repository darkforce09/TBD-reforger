# Infrastructure export

The infrastructure layers of the map export: fences and walls as cover lines, bridges and piers,
runways and helipads, and the power grid, each found among the placed entities of the world open in
[Workbench](/documentation_v2/glossary.md#workbench) and written as JSON to one folder.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Infrastructure/
├── TBD_InfrastructureExportPlugin.c  the standalone plugin that runs all four exporters in turn
├── TBD_MapExportAviation.c           runways and helipads, to `aviation.json`
├── TBD_MapExportBridges.c            bridges and piers, to `bridges_piers.json`
├── TBD_MapExportFences.c             fence and wall lines, to `fences_walls.jsonl`
└── TBD_MapExportPowerlines.c         pylons and wire spans, to `power_grid.json`
```

## How it works

Every exporter works the same way. It takes the shared context and config from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`, sweeps the world in square cells of
`m_fObjectChunkSizeM` (512 m by default) from −250 m to 1,000 m in height, and tests each entity's
prefab name by substring. A matching entity becomes one record built from its world transform and
local box. The sweep keeps no seen-entity set and does not require the entity's origin to lie in the
cell, so an entity whose box crosses a cell edge is recorded once per cell it touches. Each exporter
writes its data file and a meta file (`method`, its counts, `dataFile`, and `worldSizeM` for the
fences) into `$profile:TBD_Export/<map>/infrastructure/`, and deletes the data file when a write fails.

| Exporter | Prefab name holds | Record | Files |
|---|---|---|---|
| fences | `/walls/`, `/fences/`, `/barriers/`, `fence_`, `wall_`, `barrier_`, `sandbag` | `id`, `wallClass`, a centreline `p1` to `p2` along the longer horizontal axis, `heightM`, `thicknessM`, `prefab` | `fences_walls.jsonl` (one object per line), `fences_meta.json` |
| bridges | `/bridge`, `bridge_`, `viaduct` | `deckCenter`, `deckElevationYM`, the terrain height below as `underneathElevationYM`, `clearanceM`, `lengthM`, `widthM`, `headingDeg`, `prefab` | `bridges_piers.json` (`bridges`), `bridges_meta.json` |
| piers | `/pier`, `pier_`, `/dock`, `dock_`, `quay`, and a footprint at least three times longer than wide | `type` `pier` or `dock`, `p1`, `p2`, `widthM`, `deckElevationYM`, the water depth at `p2` as `tipWaterDepthM` | `bridges_piers.json` (`piers`) |
| runways | `runway` | `designator` (`09/27` style, from the heading), `headingDeg`, `thresholdA`, `thresholdB`, `lengthM`, `widthM`, `surface`, `prefab` | `aviation.json` (`runways`), `aviation_meta.json` |
| helipads | `helipad`, `heli_pad`, `landingzone` | `center`, `radiusM` (8 m when the box gives under 4 m), `surface`, `prefab` | `aviation.json` (`helipads`) |
| power grid | `/powerline`, `powerline_`, `pylon_`, `pole_wood`, `transformer`, `substation` | pylons: `type` (`high_voltage`, `transformer`, `distribution_pole`), `pos`, `heightM`, `prefab`; wires: `from`, `to`, `cableCount`, `distanceM`, `sagM` | `power_grid.json`, `powerlines_meta.json` |

The fence class sets a fixed thickness: `stone_wall` 0.5 m, `concrete_wall` 0.4 m, `wire_fence`
0.15 m, `barrier` 0.8 m, and `wooden_fence` 0.35 m for the rest. A wire span joins each pylon to the
nearest later pylon of the same type between 5 m and 120 m away, with 6 cables and 3.5 m of sag for
high voltage and 3 cables and 1.5 m otherwise. Heights come from `WorldEditorAPI.GetTerrainSurfaceY`.

`TBD_InfrastructureExportPlugin` builds its own config and context and runs the fence, bridge,
aviation and power grid exporters in that order; `Configure` opens the config dialog. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.
`TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs each
exporter behind `m_bExportFences`, `m_bExportBridges`, `m_bExportAviation` and
`m_bExportPowerlines`, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB` and Workbench's `WorldEditorAPI.GetTerrainSurfaceY`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads these files.
- Rules: each exporter's substring test decides its membership alone, and nothing stops two
  exporters accepting one entity; lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon, so these scripts compile only when
  Workbench loads `tbd-export`.
