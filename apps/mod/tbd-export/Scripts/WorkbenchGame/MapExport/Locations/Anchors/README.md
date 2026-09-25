# Landmark anchor export

Finds tall landmark structures (churches, lighthouses, control towers, radio masts, castles) in the
world open in [Workbench](/documentation_v2/glossary.md#workbench) and records where each stands,
the ground height under it and how high it rises, as reference points for checking a map against
the engine.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Locations/Anchors/
├── TBD_AnchorsExportPlugin.c  the standalone plugin that runs the anchor export
└── TBD_MapExportAnchors.c     `TBD_MapExportAnchors`: finds the landmarks and writes their records
```

## How it works

`TBD_MapExportAnchors.Export` takes the shared context and config from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/` and sweeps the world in square cells of
`m_fObjectChunkSizeM` (512 m by default), from −250 m to 1,000 m in height. Each entity with a
prefab name is tested by substring, and the first match sets its category and name:

| Prefab name holds | `category` | `name` |
|---|---|---|
| `church`, `cathedral`, `chapel` | `civic_landmark` | Church Tower |
| `lighthouse` | `maritime_landmark` | Lighthouse |
| `controltower`, `airport_tower`, `hangar_large` | `aviation_landmark` | Airport Control Tower |
| `radiomast`, `antenna_tower`, `radar_dome`, `broadcast` | `communications_landmark` | Radio Broadcast Tower |
| `castle`, `monument`, `fortress` | `historic_landmark` | Castle / Monument |

For each match it reads the world transform and local box: `groundElevationYM` is
`WorldEditorAPI.GetTerrainSurfaceY` at the origin, `apexElevationYM` the origin height plus the box
top, `structureHeightM` their difference (the box height when that is under 1 m), and `headingDeg`
the forward axis's yaw from 0 to 360. The query runs cell by cell and keeps no seen-entity set, so a
structure whose box crosses a cell edge is recorded once per cell it touches.

It writes `$profile:TBD_Export/<map>/anchors/verification.json`, an array of records (`id`
`anchor_<n>`, `name`, `category`, `pos`, the three heights, `headingDeg`, `prefab`), and
`anchors_meta.json` (`method` `mod-anchors-oracle-export`, `anchorCount`, `worldSizeM`,
`dataFile`); a failed write deletes `verification.json`.

`TBD_AnchorsExportPlugin` builds its own config and context and runs the export; `Configure` opens
the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no
menu. `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs
the export when `m_bExportAnchors` is set, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB` and Workbench's `WorldEditorAPI.GetTerrainSurfaceY`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads the file. The committed Everon anchors
  (`assets_v2/terrains/everon/anchors/verification.json`) share its name but not its shape: they
  follow `contracts_v2/definitions/terrain-anchors.schema.json` and are probed by hand.
- Rules: a landmark's category is decided by its prefab name alone; lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon, so these scripts compile only when
  Workbench loads `tbd-export`.

## Related documentation

- [Everon surface anchors](/assets_v2/terrains/everon/anchors/README.md) — the committed anchors
  and the gate that checks the elevation model against them.
