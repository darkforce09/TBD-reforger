# Road network export

Exports the open world's road network from [Workbench](/documentation_v2/glossary/n_to_z.md#workbench), one
JSON file per road class, and joins the classes into one junction graph in a metadata file. Each
class folder decides which entities belong to it; this folder runs them together.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/
├── Dirt/                    dirt and gravel roads, written to `roads_dirt.json`
├── Highways/                highways and arterials, written to `highways.json`; a road network probe
├── Paths/                   footpaths and trails, written to `paths.json`
├── Paved/                   asphalt and cobblestone roads, written to `roads_paved.json`
├── Runways/                 airfield runways and taxiways, written to `runways.json`
├── TBD_MapExportRoads.c     runs each class, joins their junctions, writes `roads_meta.json`
├── TBD_RoadsExportPlugin.c  the standalone plugin that runs the whole road export
└── Tracks/                  forestry and farm tracks, written to `tracks.json`
```

## How it works

```text
TBD_RoadsExportPlugin / TBD_MapExportPlugin
        |
        v
TBD_MapExportRoads.Execute
        |-- m_bExportHighways   -> TBD_MapExportHighways   -> highways.json
        |-- m_bExportPavedRoads -> TBD_MapExportPavedRoads -> roads_paved.json
        |-- m_bExportDirtRoads  -> TBD_MapExportDirtRoads  -> roads_dirt.json
        |-- m_bExportTracks     -> TBD_MapExportTracks     -> tracks.json
        |-- m_bExportPaths      -> TBD_MapExportFootpaths  -> paths.json
        |-- m_bExportRunways    -> TBD_MapExportRunways    -> runways.json
        v
junction graph over every segment's two endpoints -> roads_meta.json
```

Each class exporter sweeps the world in `m_fObjectChunkSizeM` cells, keeps the entities its
classifier accepts, turns them into segments (from the engine's road network, from spline shapes, or
by chaining loose entities), links segment endpoints that nearly meet, and hands its records back.
`TBD_MapExportRoads` then merges every endpoint of every class into junctions: an endpoint within
2.5 m of an existing junction joins it, otherwise it starts a new one. It writes `roads_meta.json`
with the segment count and total length, the node counts by degree (dead ends, two-way joints,
three-way, four-way and complex junctions), one entry per class (file, `roadClass`, whether the
config enabled it, segment count, length), every junction of degree 2 or more with its position and
segment ids, and the run time.

`TBD_RoadsExportPlugin` builds its own config and context and runs the coordinator. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.
`TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs the
coordinator when `m_bExportRoads` is set, and its attribute is commented out too.

### The road dataset format

Every class file shares one shape, written by hand as JSON text into
`$profile:TBD_Export/<map>/roads/`:

- the top level holds `type` `RoadTypeDataset`, `roadClass`, `mapName`, `worldSizeM`,
  `totalSegments`, `totalLengthM`, `bounds` (`min` and `max`) and `segments`;
- each segment holds `id` (`<class>_<n>`), `name`, `roadClass`, `widthM`, `totalLengthM`,
  `pointsCount`, `points` as `[x, y, z]` world positions, `bounds`, a `startNode` and an `endNode`
  (each a `nodeId`, a `pos` and `connectedSegmentIds`), the segment-wide `connectedSegmentIds`,
  `prefab` and `material`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `RoadNetworkManager`, `BaseRoad`, `SplineShapeEntity` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads these files: the road archive the map engine draws
  (`assets_v2/terrains/everon/roads/`) comes from `world build-roads`, which decodes the road
  topology from the game paks (`tools_v2/developer-tools/src/world_export_pipeline/roads_emit.rs`).
- Rules: each class exporter returns its records to the coordinator and keeps the shared dataset
  format; a class's classifier decides membership alone, and nothing stops two classifiers accepting
  one entity. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.

## Related documentation

- [Everon road network archive](/assets_v2/terrains/everon/roads/README.md) — the road data the map
  engine loads, and how it is built.
