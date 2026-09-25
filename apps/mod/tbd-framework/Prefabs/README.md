# Framework prefabs

The entity templates of the framework [mod](/documentation_v2/glossary.md#mod), grouped by role.
Today the addon carries one role, its system entities: the game mode and the player controller.

## Contents

```text
apps/mod/tbd-framework/Prefabs/
└── Systems/  the game mode with the framework's manager components, and the player controller
```

## How it works

The framework spawns everything else it needs (slot bodies, mission entities, vehicles) from
vanilla prefabs that the mission's aliases resolve to through `apps/mod/tbd-framework/Data/registry.json`,
so its own prefabs are only the entities that carry its scripts. The world layer
`apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/default.layer` places the game mode, and the game
mode names the player controller.

## Format

- File type: Enfusion entity templates (`.et`), plain text of the form
  `<class> : "<parent resource>" { … }`, each deriving from a vanilla prefab and holding only the
  components and properties the framework overrides.
- Resource GUID: each `.et` has a `.et.meta` whose `Name "{GUID}Prefabs/…"` line holds the GUID
  other resources refer to it by; a referenced GUID never changes.
- Naming: `TBD_<Subject>.et`, in a subfolder per role (`Systems/` for the framework's own system
  entities).
- Adding a prefab: create it in Workbench inside this addon, in the subfolder of its role, and
  commit the `.et`, its `.meta` and the rewritten `apps/mod/tbd-framework/resourceDatabase.rdb`
  together.

## Referenced by

- `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/default.layer`, which places
  `Systems/TBD_GameMode.et` by resource GUID.
- The component classes in `apps/mod/tbd-framework/Scripts/Game/TBD/`, which run only because the
  game mode prefab carries them.

## Boundaries

- Depends on: the vanilla parent prefabs from the game's data and the framework's component
  classes.
- Used by: the framework's world and, through it, its mission header.
- Rules: a referenced GUID stays stable; a prefab and its `.meta` are committed together;
  `cargo xtask mod world-boot` checks that every component on the game mode instantiates.
