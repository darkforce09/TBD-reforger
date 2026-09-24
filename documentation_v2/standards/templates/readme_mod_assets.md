**Status:** live

# README template: mod assets

**When to use:** any folder inside an Enfusion addon other than `Scripts/`: prefabs, configs,
mission headers, worlds, layouts, textures, sounds and data (`apps/mod/tbd-framework/Prefabs/`,
`apps/mod/tbd-framework/Configs/`). The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the mod assets kind adds Format and Referenced by.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Contents lists
each asset with its `.meta` file, by name or by one glob line for the pair.

````markdown
# <What the assets are, in plain words: no path, no backticks>

<One to three sentences: what the assets are for and what loads them.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/    <what it holds: a lowercase phrase, no closing period>
├── <asset file>       <what it is for>
└── <asset file>.meta  <its resource GUID>
```

## How it works

<How the engine reaches the assets: which mission header, world, prefab or script loads them, what
each asset inherits from, and what it overrides. An ASCII diagram in a text block helps here.>

## Format

- File type: <the Enfusion resource type, its text or binary format, and what it inherits from>
- Resource GUID: <where the GUID lives (the .meta file) and the rule for keeping it stable>
- Naming: <the naming convention and the subfolder per role>
- Adding an asset: <how it is created (Workbench or by hand), which files are committed together,
  and the check that follows>

## Referenced by

- <the prefab, config, layout, world, mission header or script that refers to the assets, with its
  path, and how: by resource GUID, by path or by class>

## Boundaries

- Depends on: <the vanilla resources the assets inherit from, and the script classes they name>
- Used by: <everything outside the folder that loads them, found with git grep>
- Rules: <the invariants a change must keep: stable GUIDs, files committed in pairs, Workbench
  churn>

## Related documentation

- [<document title>](/documentation_v2/mod/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/mod/tbd-framework/Prefabs/`, whose one child folder holds the two prefabs with
their `.meta` files. No document covers these prefabs, so the sample has no Related documentation.
The sample sits in a fenced block, so no gate reads it as a README; the folder's own README.md is
written from the same files and may differ.

````markdown
# Framework prefabs

The entity templates the framework boots with: the game mode that carries every framework manager
component, and the player controller it hands each player.

## Contents

```text
apps/mod/tbd-framework/Prefabs/
└── Systems/  the game mode prefab and the player controller prefab it names
```

## How it works

A mission header names a world; the world's layer places the framework's game mode from its
prefab; the game mode prefab carries the framework's manager components and names the player
controller prefab each connecting player gets. Each prefab derives from a vanilla prefab and holds
only what the framework changes.

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent
worlds/TBD_Dev_POC_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_GameMode.et
Prefabs/Systems/TBD_GameMode.et ──PlayerControllerPrefab──▶ Prefabs/Systems/TBD_PlayerController.et
```

## Format

- File type: Enfusion entity templates (`.et`), plain text of the form
  `<class> : "<parent resource>" { … }`, holding the component blocks and properties the prefab
  overrides. `TBD_GameMode.et` derives from the vanilla `GameMode_Plain.et`, and
  `TBD_PlayerController.et` from `DefaultPlayerControllerMP.et`.
- Resource GUID: each `.et` has a `.et.meta` file whose `Name "{GUID}Prefabs/…"` line holds the
  resource GUID other resources refer to it by; a referenced GUID never changes.
- Naming: `TBD_<Subject>.et`, in a subfolder per role; `Systems/` holds the framework's own
  entities.
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
  through its world, `{F652B97A6F497348}worlds/TBD_Dev_POC.ent`.
- Everon's exported type inventory lists the game mode prefab by resource name
  (`assets_v2/terrains/everon/objects/type-inventory.json`).

## Boundaries

- Depends on: the vanilla parent prefabs `{1B76F75A3175E85C}Prefabs/MP/Modes/Plain/GameMode_Plain.et`
  and `{225E51284CC95CFA}Prefabs/Characters/Core/DefaultPlayerControllerMP.et` from the game's
  data, and the component classes in `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: the framework's world layer and, through it, its mission header, as listed above.
- Rules: a script component runs only on an entity whose prefab carries it, so a new manager
  component is added to `TBD_GameMode.et`; GUIDs stay stable; a prefab and its `.meta` are
  committed together.
````
