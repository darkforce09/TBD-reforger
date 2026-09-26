# TBD Dev POC world layer

The one entity layer of the TBD Dev POC world: it places the framework's game mode and the three
vanilla managers the game mode needs on Everon.

## Contents

```text
apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/
└── default.layer  the default layer: the TBD game mode, faction, loadout and AI world managers
```

## Format

- File type: an [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) world layer (`.layer`), plain
  text, one `<class> <name> : "<prefab resource>" { coords x y z }` block per placed entity. The
  layer places four entities, all at `6400 0 6400`:
  - `SCR_BaseGameMode TBD_GameMode`, from `{7A5B8572ECC15707}Prefabs/Systems/TBD_GameMode.et`;
  - `SCR_FactionManager FactionManager`, from vanilla `FactionManager_Editor.et`;
  - `SCR_LoadoutManager LoadoutManager`, from vanilla `LoadoutManager_Editor.et`;
  - `SCR_AIWorld AIWorld`, from vanilla `SCR_AIWorld_Eden.et`.
- Resource GUID: none; the layer is loaded by its place beside the world, as
  `<world>_Layers/default.layer`, and carries no `.meta`.
- Naming: the folder is named after its world, `TBD_Dev_POC.ent`; `default.layer` is the layer
  Enfusion loads with the world.
- Adding an entity: place it in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) with this world
  open and save, which rewrites the layer; hand edits keep the block syntax and a prefab's
  `{GUID}path` resource name.

## Referenced by

- `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent`, whose layers these are, by folder name.
- Through the world, the [mission header](/documentation_v2/glossary/g_to_m.md#mission-header)
  `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf` and every server that boots it.

## Boundaries

- Depends on: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` and the vanilla
  `Prefabs/MP/Managers/Factions/FactionManager_Editor.et`,
  `Prefabs/MP/Managers/Loadouts/LoadoutManager_Editor.et` and `Prefabs/AI/SCR_AIWorld_Eden.et`
  from the game's data.
- Used by: the TBD Dev POC world when it loads; under `apps/mod/tbd-framework/Scripts/Game/TBD/`,
  `TBD_SpawnManager`, `TBD_DynamicSpawner` and `TBD_SpectatorTargets` read the faction manager,
  and `TBD_SafestartManager` and the modded respawn system read the AI world.
- Rules: the game mode entity comes from `TBD_GameMode.et`, never an inline copy, so its manager
  components stay in one prefab; the layer places no `RadioManagerEntity`, which the radio system
  reports at boot and works around with its own channel table; `cargo xtask mod world-boot` boots
  the world and checks the game mode's component roll-call.
