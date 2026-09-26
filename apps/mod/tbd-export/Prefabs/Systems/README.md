# Export game mode prefab

The game mode the export world places: a plain vanilla game mode that carries the runtime road
exporter as a component, so playing the export
[mission header](/documentation_v2/glossary/g_to_m.md#mission-header) runs the road export.

## Contents

```text
apps/mod/tbd-export/Prefabs/Systems/
├── TBD_Export_GameMode.et       the export game mode: `TBD_RoadExportComponent` and no auto respawn
└── TBD_Export_GameMode.et.meta  its resource GUID, `{C3D4E5F6A7B80001}`
```

## How it works

`TBD_Export_GameMode.et` is an `SCR_BaseGameMode` derived from the vanilla
`{1B76F75A3175E85C}Prefabs/MP/Modes/Plain/GameMode_Plain.et`. It adds a `TBD_RoadExportComponent`
block, whose class lives in `apps/mod/tbd-export/Scripts/Game/TBD/Export/`, overrides the vanilla
`SCR_RespawnSystemComponent` with an empty loading layout, and switches off automatic player
respawn (`m_bAutoPlayerRespawn 0`) and faction changes (`m_bAllowFactionChange 0`).

## Format

- File type: an Enfusion entity template (`.et`), plain text
  `SCR_BaseGameMode : "<parent resource>" { … }` holding the component blocks and properties the
  prefab overrides.
- Resource GUID: `TBD_Export_GameMode.et.meta` holds
  `Name "{C3D4E5F6A7B80001}Prefabs/Systems/TBD_Export_GameMode.et"`; the world layer refers to the
  prefab by that GUID, so it never changes.
- Naming: `TBD_Export_<Subject>.et` for the export addon's system entities.
- Adding a prefab: create it in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) inside this
  addon, which writes the `.meta` with a new GUID, and commit the `.et` and its `.meta` together.

## Referenced by

- `apps/mod/tbd-export/worlds/TBD_Export_Everon_Layers/default.layer` places the game mode by
  resource GUID: `SCR_BaseGameMode TBD_Export_GameMode : "{C3D4E5F6A7B80001}Prefabs/Systems/TBD_Export_GameMode.et"`,
  at `6400 0 6400`.
- `TBD_RoadExportComponent` attaches to it by class, from
  `apps/mod/tbd-export/Scripts/Game/TBD/Export/TBD_RoadExportComponent.c`.

## Boundaries

- Depends on: the vanilla parent prefab `GameMode_Plain.et` and `SCR_RespawnSystemComponent` from
  the game's data; the component class `TBD_RoadExportComponent`.
- Used by: the export world's layer, and through it the export mission header in
  `apps/mod/tbd-export/Missions/`.
- Rules: the GUID `{C3D4E5F6A7B80001}` stays stable; the prefab and its `.meta` are committed
  together; the prefab carries no framework component, since the export addon does not depend on
  `tbd-framework` (`apps/mod/tbd-export/addon.gproj`).
