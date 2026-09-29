# TBD Dev POC Arland world layer

The one entity layer of the TBD Dev POC Arland world: it places the framework's game mode and the
three vanilla managers the game mode needs on Arland, the same set the Everon world's layer places.

## Contents

```text
apps/mod/tbd-framework/worlds/TBD_Dev_POC_Arland_Layers/
└── default.layer  the default layer: the TBD game mode, faction, loadout and AI world managers
```

## Format

- File type: an [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) world layer (`.layer`), plain
  text, one `<class> <name> : "<prefab resource>" { coords x y z }` block per placed entity. The
  layer places four entities, all at `2048 0 2048`, the centre of the 4,096 m Arland terrain:
  - `SCR_BaseGameMode TBD_GameMode`, from `{7A5B8572ECC15707}Prefabs/Systems/TBD_GameMode.et`;
  - `SCR_FactionManager FactionManager`, from vanilla `FactionManager_Editor.et`;
  - `SCR_LoadoutManager LoadoutManager`, from vanilla `LoadoutManager_Editor.et`;
  - `SCR_AIWorld AIWorld`, from vanilla `{01DC74137CFDDB6A}Prefabs/AI/SCR_AIWorld_Arland.et`, whose
    navmesh files are Arland's.
- Resource GUID: none; the layer is loaded by its place beside the world, as
  `<world>_Layers/default.layer`, and carries no `.meta`.
- Naming: the folder is named after its world, `TBD_Dev_POC_Arland.ent`; `default.layer` is the
  layer Enfusion loads with the world.
- Adding an entity: place it in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) with this
  world open and save, which rewrites the layer; hand edits keep the block syntax and a prefab's
  `{GUID}path` resource name.

## Referenced by

- `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Arland.ent`, whose layers these are, by folder name.
- Through the world, the [mission header](/documentation_v2/glossary/g_to_m.md#mission-header)
  `apps/mod/tbd-framework/Missions/TBD_Dev_POC_Arland.conf` and every server that boots it.

## Boundaries

- Depends on: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` and the vanilla
  `Prefabs/MP/Managers/Factions/FactionManager_Editor.et`,
  `Prefabs/MP/Managers/Loadouts/LoadoutManager_Editor.et` and `Prefabs/AI/SCR_AIWorld_Arland.et`
  from the game's data.
- Used by: the TBD Dev POC Arland world when it loads; the framework scripts read these managers
  exactly as they do on Everon (see `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/README.md`).
- Rules: the layer places the same entities as `worlds/TBD_Dev_POC_Layers/default.layer`, with
  Arland's AI world and Arland coordinates, so the game mode behaves the same on both terrains; the
  game mode entity comes from `TBD_GameMode.et`, never an inline copy; the layer places no
  `RadioManagerEntity`, which the radio system reports at boot and works around with its own
  channel table.
