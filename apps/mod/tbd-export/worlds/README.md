# Export worlds

The export addon's world: a sub-scene of the vanilla Eden (Everon) terrain that adds only the export
game mode and an AI world, so the runtime road exporter can read Everon's road network when the
export [mission header](/documentation_v2/glossary.md#mission-header) is played.

## Contents

```text
apps/mod/tbd-export/worlds/
├── TBD_Export_Everon.ent       the Everon export world, a sub-scene of vanilla Eden
├── TBD_Export_Everon.ent.meta  its resource GUID, `{C3D4E5F6A7B80002}`
└── TBD_Export_Everon_Layers/   the world's layer: the export game mode and the AI world
```

## How it works

`TBD_Export_Everon.ent` is a `SubScene` whose only line is its parent,
`{853E92315D1D9EFE}worlds/Eden/Eden.ent`: it inherits the whole vanilla terrain and every placed
object. Its layer adds the export game mode and `SCR_AIWorld_Eden`; the AI world builds the road
network manager that `TBD_RoadExportComponent` queries.

```text
Missions/TBD_Export_Everon.conf ──World──▶ TBD_Export_Everon.ent ──Parent──▶ {853E92315D1D9EFE}worlds/Eden/Eden.ent
                                           TBD_Export_Everon_Layers/default.layer ──▶ export game mode + AI world
```

The Workbench map-export plugins in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/` work on
whichever world is open in [Workbench](/documentation_v2/glossary.md#workbench) and need neither
this world nor its game mode.

## Format

- File type: an Enfusion world (`.ent`), plain text; a `SubScene { Parent "<world>" }` block
  inherits the parent world, and the `<world>_Layers/` folder beside it holds what it adds.
- Resource GUID: `TBD_Export_Everon.ent.meta` holds
  `Name "{C3D4E5F6A7B80002}worlds/TBD_Export_Everon.ent"`; the mission header names the world by
  it, so it never changes.
- Naming: `TBD_Export_<Terrain>.ent`, one export world per terrain.
- Adding a world: create a sub-scene of the terrain's vanilla world in Workbench inside this addon,
  place the export game mode and the terrain's AI world, and commit the `.ent`, its `.meta` and its
  layer folder together.

## Referenced by

- `apps/mod/tbd-export/Missions/TBD_Export_Everon.conf` names the world by resource GUID:
  `World "{C3D4E5F6A7B80002}worlds/TBD_Export_Everon.ent"`.

## Boundaries

- Depends on: the vanilla world `worlds/Eden/Eden.ent` and AI world prefab from the game's data;
  the export game mode in `apps/mod/tbd-export/Prefabs/Systems/`.
- Used by: the export mission header in `apps/mod/tbd-export/Missions/`.
- Rules: the world changes nothing of the vanilla terrain, so exports read Everon as the game ships
  it; the GUID `{C3D4E5F6A7B80002}` stays stable; the `.ent`, its `.meta` and the layer folder are
  committed together.
