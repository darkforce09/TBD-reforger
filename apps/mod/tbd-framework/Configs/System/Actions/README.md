# Framework input actions

The framework [mod](/documentation_v2/glossary/g_to_m.md#mod)'s own key actions: each file binds one
named action to one keyboard key, and the scripts listen for the action by name while its input
context is active.

## Contents

```text
apps/mod/tbd-framework/Configs/System/Actions/
├── *.conf.meta               each action's resource GUID, `{7BD1A70000000742}` to `…749` and `…B50`
├── TBD_AdminMenu.conf        F8: toggle the admin screen
├── TBD_MissionCycle.conf     F6: fetch the mission list, then step through it
├── TBD_MissionLoad.conf      F7: ask the server to request a deployment of the highlighted mission
├── TBD_MissionSelector.conf  F9: toggle the mission selector screen
├── TBD_SpecFree.conf         F: spectator free camera
├── TBD_SpecNext.conf         Right arrow: spectate the next target
├── TBD_SpecPrev.conf         Left arrow: spectate the previous target
├── TBD_SpecRoster.conf       Tab: toggle the spectator roster
└── TBD_SpecView.conf         V: toggle first-person spectating
```

## How it works

Each action belongs to a context in `apps/mod/tbd-framework/Configs/System/ActionContext/`: the
four admin and mission keys to `TBD_BrowserContext`, the five spectator keys to
`TBD_SpectatorContext`. The scripts register a listener per action name with
`AddActionListener(<name>, EActionTrigger.DOWN, …)`:

- the mission browser's modded `SCR_PlayerController`, on the local client, for the F6 to F9
  keys: F6 prints the next line of the server's mission list to the log, fetching the list on
  the first press; F7 asks the server to request a deployment of the highlighted mission; F8 calls
  `TBD_AdminClient.Toggle`; F9 calls `TBD_MissionSelectorScreen.Toggle`;
- `TBD_SpectatorController`, for the spectator keys, which act only while spectating. Every
  spectator key is also a click on the roster screen.

## Format

- File type: Enfusion config (`.conf`), plain text of the form `Action <name> { InputSource
  InputSourceSum { Sources { InputSourceValue { FilterPreset "down" Input "keyboard:KC_<key>"
  Filter InputFilterDown { } } } } }`: one keyboard key, firing on key down. No action binds a
  mouse button or a gamepad input.
- Resource GUID: each `.conf` has a `.conf.meta` whose `Name "{GUID}Configs/…"` line holds its
  GUID, in the hand-assigned `7BD1A700000007xx` range, with `…B50` for `TBD_MissionSelector.conf`;
  a GUID never changes once assigned.
- Naming: `TBD_<Action>.conf`, the file name equal to the action name the scripts listen for;
  spectator actions start with `TBD_Spec`.
- Adding an action: write the `.conf` and its `.meta` from a sibling, add the action name to its
  context's `ActionRefs`, commit the three files together, open the addon in Workbench once so
  `resourceDatabase.rdb` lists the new file, and register a listener in the owning script.

## Referenced by

- `apps/mod/tbd-framework/Configs/System/ActionContext/TBD_BrowserContext.conf` and
  `TBD_SpectatorContext.conf` list the actions by name.
- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionBrowser.c` listens for
  `TBD_MissionCycle`, `TBD_MissionLoad`, `TBD_AdminMenu` and `TBD_MissionSelector`.
- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/TBD_SpectatorController.c` listens for
  the five `TBD_Spec*` actions.
- `apps/mod/tbd-framework/resourceDatabase.rdb` registers every file.

## Boundaries

- Depends on: the engine's input manager and key names (`KC_*`).
- Used by: the contexts and scripts listed above.
- Rules: an action name, a context's reference to it and the script's listener change together;
  the `.conf` and its `.meta` are committed together; `TBD_MissionSelector.conf.meta` declares
  `LayoutResourceClass` for every platform but PC where its siblings declare `CONFResourceClass`,
  so a copy is taken from another sibling.

## Related documentation

- [Spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
  — the spectator controls.
- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the admin screen F8 opens.
