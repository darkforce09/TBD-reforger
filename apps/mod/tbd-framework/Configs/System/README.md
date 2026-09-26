# Framework system configs

The engine-level configs of the framework [mod](/documentation_v2/glossary/g_to_m.md#mod): the menu
presets that bind each framework screen to its layout and script class, and the input contexts and
key actions its screens and spectator camera listen to.

## Contents

```text
apps/mod/tbd-framework/Configs/System/
├── ActionContext/          the browser and spectator input contexts that group the key actions
├── Actions/                the nine key actions: F6 to F9 for admin and missions, five for spectating
├── chimeraMenus.conf       the six framework menu presets, each a layout plus a screen class
└── chimeraMenus.conf.meta  its resource GUID, `{7BD1A70000000703}`
```

## How it works

`chimeraMenus.conf` is a `MenuManager` config with one `MenuPreset` per framework screen. The
engine's menu manager reads it because `apps/mod/tbd-framework/addon.gproj` names it in the
`MenuConfigs` of both its `PC` and `HEADLESS` configurations, after vanilla's
`{C747AFB6B750CE9A}Configs/System/chimeraMenus.conf`: an addon's `MenuConfigs` list replaces
vanilla's, so it names vanilla's config as well to keep the vanilla menus. Each preset name is also
a `modded enum ChimeraMenuPreset` value in the screen's script, and `TBD_MenuStack` opens screens by
that value.

| Preset | Layout | Class |
|---|---|---|
| `TBD_UIShell` | `UI/layouts/Common/TBD_ScreenShell.layout` | `TBD_ShellScreen` |
| `TBD_Spectator` | `UI/layouts/Common/TBD_ScreenShell.layout` | `TBD_SpectatorScreen` |
| `TBD_UIBriefing` | `UI/layouts/Session/Briefing/TBD_BriefingScreen.layout` | `TBD_BriefingScreen` |
| `TBD_UILobby` | `UI/layouts/Session/Lobby/TBD_LobbyScreen.layout` | `TBD_LobbyScreen` |
| `TBD_UIAdmin` | `UI/layouts/Common/TBD_ScreenShell.layout` | `TBD_AdminScreen` |
| `TBD_UIMissionSelector` | `UI/layouts/Session/MissionSelector/TBD_MissionSelector.layout` | `TBD_MissionSelectorScreen` |

The layout paths are relative to `apps/mod/tbd-framework/`. The key actions in `Actions/` belong to
the contexts in `ActionContext/`; scripts activate a context and listen for its actions by name.

## Format

- File type: Enfusion configs (`.conf`), plain text of the form `<Class> <name> { … }`:
  `MenuManager` here, `ActionContext` and `Action` in the subfolders.
- Resource GUID: each `.conf` has a `.conf.meta` whose `Name "{GUID}Configs/…"` line holds its GUID.
  The framework's configs use the hand-assigned `7BD1A70000000xxx` range; a GUID that
  `addon.gproj`, a config or the resource database names never changes.
- Naming: `chimeraMenus.conf` keeps the engine's name for the menu config; the input configs are
  `TBD_`-prefixed and grouped by type in `ActionContext/` and `Actions/`.
- Adding a menu preset: add a `MenuPreset` block here, a `modded enum ChimeraMenuPreset` value of
  the same name in the screen's script, and the layout it names; open the addon in Workbench once so
  `apps/mod/tbd-framework/resourceDatabase.rdb` lists any new resource, and commit the rdb with it.

## Referenced by

- `apps/mod/tbd-framework/addon.gproj` names `{7BD1A70000000703}Configs/System/chimeraMenus.conf` by
  resource GUID in `MenuConfigs`.
- The screen scripts in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` (`TBD_ShellScreen.c`)
  and `apps/mod/tbd-framework/Scripts/Game/TBD/Session/` (the admin, briefing, lobby, mission
  selector and spectator screens) declare the preset names, and `TBD_MenuStack` opens them.
- The presets name their layouts in `apps/mod/tbd-framework/UI/layouts/` by resource GUID.

## Boundaries

- Depends on: vanilla's menu config and `ChimeraMenuPreset` enum; the layouts in
  `apps/mod/tbd-framework/UI/layouts/`; the screen classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: the framework's addon project, screens and spectator camera, as listed above.
- Rules: a preset name, its enum value and its screen class change together; the `MenuConfigs`
  list keeps vanilla's config first; every `.conf` is committed with its `.meta`; a new resource is
  not visible to the engine until Workbench has rewritten the resource database.

## Related documentation

- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — the specification of
  each framework screen these presets open.
