# Export world layer

The one layer of the Everon export world: it places the export game mode and Eden's AI world on top
of the vanilla Eden terrain.

## Contents

```text
apps/mod/tbd-export/worlds/TBD_Export_Everon_Layers/
└── default.layer  places the export game mode and `SCR_AIWorld_Eden` at the world centre
```

## Format

- File type: an Enfusion layer (`.layer`), plain text, one `<class> <name> : "<prefab>" { … }`
  block per placed entity. It holds two:
  - `SCR_BaseGameMode TBD_Export_GameMode` from
    `{C3D4E5F6A7B80001}Prefabs/Systems/TBD_Export_GameMode.et`, at `coords 6400 0 6400`;
  - `SCR_AIWorld AIWorld` from the vanilla `{70CCCF16487C927F}Prefabs/AI/SCR_AIWorld_Eden.et`, at
    the same coordinates. The runtime road exporter reads the road network from this AI world.
- Resource GUID: a layer has no `.meta`; the world `TBD_Export_Everon.ent` loads it from the
  `TBD_Export_Everon_Layers/` folder beside it.
- Naming: Workbench names the folder `<world>_Layers/` and the default layer `default.layer`.
- Adding an entity: place it in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) with the
  export world open and save the world, which rewrites the layer.

## Referenced by

- `apps/mod/tbd-export/worlds/TBD_Export_Everon.ent`, by folder name.

## Boundaries

- Depends on: `apps/mod/tbd-export/Prefabs/Systems/TBD_Export_GameMode.et` and the vanilla AI
  world prefab.
- Used by: the export world, and through it the export
  [mission header](/documentation_v2/glossary/g_to_m.md#mission-header).
- Rules: the layer keeps the AI world, without which `TBD_RoadExportComponent` finds no road
  network manager and exports nothing; Workbench writes the file.
