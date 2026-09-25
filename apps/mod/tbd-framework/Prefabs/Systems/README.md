# Framework system prefabs

The entity templates the framework [mod](/documentation_v2/glossary.md#mod) boots with: the game
mode that carries every framework manager component, and the player controller it hands each
player.

## Contents

```text
apps/mod/tbd-framework/Prefabs/Systems/
├── TBD_GameMode.et               the game mode, carrying the framework's manager components
├── TBD_GameMode.et.meta          the game mode's resource GUID, `{7A5B8572ECC15707}`
├── TBD_PlayerController.et       the player controller each player gets, two spawn requests off
└── TBD_PlayerController.et.meta  the player controller's resource GUID, `{1A1ABD939E1E8423}`
```

## How it works

A [mission header](/documentation_v2/glossary.md#mission-header) names a world; the world's layer
places the game mode from `TBD_GameMode.et`; the game mode carries the framework's manager
components and names `TBD_PlayerController.et` as the controller each connecting player gets. Each
prefab derives from a vanilla prefab and holds only what the framework changes. Enfusion names a
resource by its path inside the addon:

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent
worlds/TBD_Dev_POC_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_GameMode.et
Prefabs/Systems/TBD_GameMode.et ──PlayerControllerPrefab──▶ Prefabs/Systems/TBD_PlayerController.et
```

`TBD_GameMode.et` derives from vanilla's plain game mode and adds eleven framework components; it
also sets `m_bAutoDeploy 0` on `TBD_SpawnManager`, blanks `m_sLoadingLayout` on the vanilla
`SCR_RespawnSystemComponent` so no loading splash is built that nothing would tear down, and turns
off `m_bAutoPlayerRespawn` and `m_bAllowFactionChange`. `TBD_PlayerController.et` derives from
vanilla's multiplayer player controller and switches off `SCR_FreeSpawnRequestComponent` and
`SCR_SpawnPointRespawnRequestComponent`, which leaves possession the only spawn request a framework
world accepts; the framework's modded `SCR_PlayerController` scripts and their RPCs run on this
controller because its class is `SCR_PlayerController`.

## Format

- File type: Enfusion entity templates (`.et`), plain text of the form
  `<class> : "<parent resource>" { … }`, holding the component blocks and properties the prefab
  overrides; each derives from the vanilla prefab named under Boundaries.
- Resource GUID: each `.et` has a `.et.meta` file whose `Name "{GUID}Prefabs/…"` line holds the
  resource GUID other resources refer to it by; a referenced GUID never changes.
- Naming: `TBD_<Subject>.et`. This folder holds the framework's own system entities; prefabs of
  another role go in a sibling folder under `apps/mod/tbd-framework/Prefabs/`.
- Adding a prefab: create it in Workbench inside this addon, which writes the `.meta` with a new
  GUID, and commit the `.et` and its `.meta` together.

## Referenced by

- `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/default.layer` places the game mode by resource
  GUID: `SCR_BaseGameMode TBD_GameMode : "{7A5B8572ECC15707}Prefabs/Systems/TBD_GameMode.et"`.
- `TBD_GameMode.et` names the player controller by resource GUID in its `PlayerControllerPrefab`
  property.
- The framework's manager components attach to the game mode by class: `TBD_FrameworkManager`,
  `TBD_SpawnManager`, `TBD_SafestartManager`, `TBD_LoadoutEquipComponent`,
  `TBD_SpectatorComponent`, `TBD_LobbyComponent`, `TBD_PlayAreaComponent`, `TBD_MarkerComponent`,
  `TBD_RadioComponent`, `TBD_ObjectivesComponent` and `TBD_PreSlotComponent` are component blocks
  of `TBD_GameMode.et`, from the classes in `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- The mission header `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf` reaches the game mode
  through its world, `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent`, which it names as the
  resource `{F652B97A6F497348}worlds/TBD_Dev_POC.ent`.
- Everon's exported type inventory lists the game mode prefab by resource name
  (`assets_v2/terrains/everon/objects/type-inventory.json`).

## Boundaries

- Depends on: the vanilla parent prefabs `{1B76F75A3175E85C}Prefabs/MP/Modes/Plain/GameMode_Plain.et`
  and `{225E51284CC95CFA}Prefabs/Characters/Core/DefaultPlayerControllerMP.et` from the game's
  data, and the component classes in `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: the framework's world layer and, through it, its mission header, as listed above.
- Rules: a script component runs only on an entity whose prefab carries it, so a new manager
  component is added to `TBD_GameMode.et`, and `cargo xtask mod world-boot` fails when a listed
  component's class does not resolve or does not instantiate; a referenced GUID stays stable; a
  prefab and its `.meta` are committed together.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the one-life rule the spawn
  settings serve.
