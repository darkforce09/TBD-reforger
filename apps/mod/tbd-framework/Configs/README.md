# Framework configs

The Enfusion config files of the framework [mod](/documentation_v2/glossary.md#mod): the menu
presets of its screens and the input contexts and key actions its scripts listen to.

## Contents

```text
apps/mod/tbd-framework/Configs/
└── System/  the menu presets, input contexts and key actions the engine and the screens read
```

## How it works

Enfusion resolves a config by its resource path inside the addon, so the framework's configs sit
under `System/`, where vanilla keeps its own system configs. The addon project names the menu
config there in its `MenuConfigs`; the input contexts and actions are reached by name from the
screen and spectator scripts. Each child README lists the files and who reads them.

## Format

- File type: Enfusion configs (`.conf`), plain text of the form `<Class> <name> { … }`, each beside
  a `.conf.meta` that holds its resource GUID.
- Resource GUID: hand-assigned in the `7BD1A70000000xxx` range; a GUID never changes once a project
  file, config or script names it.
- Naming: engine-named configs keep the engine's file name (`chimeraMenus.conf`); the framework's
  own configs are `TBD_`-prefixed and grouped by config type in subfolders.
- Adding a config: place it in the subfolder of its type with its `.meta`, open the addon in
  Workbench once so `apps/mod/tbd-framework/resourceDatabase.rdb` lists it, and commit the three
  files together.

## Referenced by

- `apps/mod/tbd-framework/addon.gproj`, which names `System/chimeraMenus.conf` by resource GUID.
- The screen, mission browser and spectator scripts under
  `apps/mod/tbd-framework/Scripts/Game/TBD/`, which open menu presets and activate input contexts by
  name.

## Boundaries

- Depends on: the vanilla menu and input systems, the layouts in `apps/mod/tbd-framework/UI/` and
  the screen classes in `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: the framework's addon project and scripts.
- Rules: every config is committed with its `.meta`; a new config reaches the engine only after
  Workbench rewrites `apps/mod/tbd-framework/resourceDatabase.rdb`.
