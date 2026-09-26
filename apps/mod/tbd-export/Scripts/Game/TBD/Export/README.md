# Runtime road network export

A game mode component that, once the export world is running, reads Everon's road network from the
engine's AI road graph and writes it as one JSON file per road class plus a junction summary. It is
the game-side counterpart of the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) road plugins,
and it runs when the export [mission header](/documentation_v2/glossary/g_to_m.md#mission-header) is
played.

## Contents

```text
apps/mod/tbd-export/Scripts/Game/TBD/Export/
├── TBD_RoadClassifier.c       `TBD_ERoadLayer` and the rules that put a road in one of six classes
├── TBD_RoadExportComponent.c  the game mode component: finds the roads, links them, writes the files
├── TBD_RoadExportJson.c       `TBD_RoadExportJson`: JSON string escaping and checked file writes
├── TBD_RoadExportPaths.c      `TBD_RoadExportPaths`: creates and names the output folders
└── TBD_RoadRecords.c          the segment and junction records the component fills and writes
```

## How it works

`TBD_RoadExportComponent` is an `SCR_BaseGameModeComponent`; the export game mode prefab
(`apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et`) carries it. `OnPostInit` schedules
`ExecuteExport` 500 ms later on the call queue, so the AI world has built its road graph first.
The export runs once:

1. It takes the map name `everon` and a 12,800 m world, both fixed in the code, and the
   `RoadNetworkManager` of the game's `ChimeraAIWorld`; without either it logs an error and stops.
2. Entity pass: it queries every entity in the world box and, for each whose class name contains
   `Road`, asks the road network for the closest road to its origin. A road within 30 m that no
   earlier probe took becomes one segment: its `BaseRoad` points and width (6 m when the engine
   reports under 1 m), classified by the entity's prefab name.
3. Grid pass: it probes the centre of every 200 m cell (64 × 64 probes) for the closest road within
   150 m and adds each road not yet taken, classified by width alone.
4. Linking: inside each class, segment ends within 5 m share a node id and list each other as
   connected. Across all classes, every segment end joins the first junction within 2.5 m or
   starts a new one.
5. Writing: each class goes to its file under `$profile:TBD_Export/everon/roads/`, then
   `roads_meta.json` summarises the run.

`TBD_RoadClassifier.Classify` rejects vegetation, props, signs, fences and power lines, then tests
the material and prefab name for runways, paths, tracks, dirt roads, highways and paved roads in
that order, and falls back on the width: 7.5 m and over is a highway, 5 m a paved road, 3.5 m a
dirt road, 2 m a track, anything narrower a path. The component passes an empty material, so only
the prefab-name and width rules apply, and an entity-pass road the classifier rejects is kept as a
paved road.

| Class | File | `roadClass` | Segment id |
|---|---|---|---|
| highway | `highways.json` | `highway_paved` | `highway_<n>` |
| paved | `roads_paved.json` | `road_paved` | `road_paved_<n>` |
| dirt | `roads_dirt.json` | `road_dirt` | `road_dirt_<n>` |
| track | `tracks.json` | `track` | `track_<n>` |
| path | `paths.json` | `path` | `path_<n>` |
| runway | `runways.json` | `runway` | `runway_<n>` |

### Output format

Each class file is written by hand as JSON text, flushed every 8,000 characters:

- the top level holds `type` `RoadTypeDataset`, `roadClass`, `mapName`, `worldSizeM`,
  `totalSegments`, `totalLengthM`, `bounds` (`min` and `max`) and `segments`;
- each segment holds `id`, `name`, `roadClass`, `widthM`, `totalLengthM`, `pointsCount`, `points`
  as `[x, y, z]` world positions, `bounds`, a `startNode` and an `endNode` (each a `nodeId`, a
  `pos` and `connectedSegmentIds`), the segment-wide `connectedSegmentIds`, `prefab` and
  `material` (always empty).

`roads_meta.json` holds `type` `RoadNetworkMetadataManifest`, `mapName`, `worldSizeM`, the segment
count and network length in metres and kilometres, a `topology` block counting junctions by degree
(dead ends, two-way joints, three-way, four-way and complex), a `layers` block with each class's
file, `roadClass`, segment count and length, the `junctions` of degree 2 or more (`id`, `pos`,
`degree`, `connectedSegments`), and `elapsedMs`.

These are the file names and the dataset shape the Workbench road export in
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/` writes into the same folder, so
whichever runs last replaces the other's files.

## Authority

- Server: everything. `OnPostInit` returns without scheduling the export when
  `RplSession.Mode()` is `RplMode.Client`, so the export runs on a dedicated or listen server and
  in Workbench's play mode.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `SCR_BaseGameModeComponent`, `ChimeraAIWorld`, `RoadNetworkManager`,
  `BaseRoad`, `BaseWorld.QueryEntitiesByAABB` and `FileIO`; the AI world that
  `apps/mod/tbd-export/worlds/TBD_Export_Everon_Layers/default.layer` places.
- Used by: `apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et`, which carries the
  component by class. No committed tool reads the files: the road archive the map engine draws
  (`assets_v2/terrains/everon/roads/`) comes from `world build-roads`, which decodes the road
  topology from the game paks (`tools_v2/developer-tools/src/world_export_pipeline/roads_emit.rs`).
- Rules: the scripts use only vanilla classes and their own `TBD_Road*` helpers, never the
  Workbench `TBD_MapExport*` classes, because a game or server compiles `Scripts/Game/` without
  `Scripts/WorkbenchGame/`; they write only below `$profile:TBD_Export/`; lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`), so these scripts compile only when
  Workbench or a game loads `tbd-export`.

## Related documentation

- [Everon road network archive](/assets_v2/terrains/everon/roads/README.md) — the road data the map
  engine loads, and how it is built.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  how the runtime road export sits beside the Workbench layers.
