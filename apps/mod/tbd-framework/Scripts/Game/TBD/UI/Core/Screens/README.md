# Screens and the screen stack

How a TBD screen opens, takes input and closes: the menu presets, the screen base every screen
derives from, the stack that keeps one screen per preset and gives input and focus to the top one,
and the two screen shells.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Screens/
├── ChimeraMenuPreset.c  ChimeraMenuPreset: every TBD menu preset, one modded enum for the framework
├── TBD_DockScreen.c     TBD_DockScreen: base of the dock screens; mounts the shared bars, routes tabs
├── TBD_MenuBase.c       TBD_MenuBase: the ChimeraMenuBase every TBD screen derives from
├── TBD_MenuStack.c      TBD_MenuStack: the screen stack; one per preset, input and focus on top
└── TBD_ShellScreen.c    TBD_ShellScreen: header, one list, one primary action; preset TBD_UIShell
```

## How it works

A preset is a member of the modded `ChimeraMenuPreset`, bound to a layout and a screen class in
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`. `TBD_MenuStack.Open` opens it through
the engine's `MenuManager`, and `TBD_MenuBase.OnMenuOpen` registers the screen with the stack
before the subclass's `OnScreenOpen` runs; every close path reaches `OnMenuClose`, which pops the
stack. Only the top screen re-arms its input context each frame and gets `OnScreenUpdate`, and
after every push and pop the new top seeds focus with `FocusDefault()`. `TBD_DockScreen` mounts
sub-layouts into named docks and the two session bars; `TBD_ShellScreen` binds the list shell's
title, list, status line and primary action.

## Authority

- Server: nothing.
- Client: everything; the screens run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `ChimeraMenuBase`, `MenuManager`, `InputManager` and widget API;
  `TBD_SessionTopBar`, `TBD_SessionBottomBar` and `TBD_ESessionTab` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/SessionChrome/`; `TBD_ListBox` and
  `TBD_UIButton` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/`; `TBD_UILayouts`
  and `TBD_UITheme` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`.
- Used by: every screen under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`, which subclass
  the bases and open, replace and close screens through `TBD_MenuStack`; `TBD_LobbyStage`, which
  resets the stack when a world starts; `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`,
  which names the presets and `TBD_ShellScreen`.
- Rules: `ChimeraMenuPreset` member names and `TBD_ShellScreen` are named by configs and stay
  frozen; subclasses override the `OnScreen*` hooks, never `OnMenu*`; lines added to a script stay
  ASCII, and `cargo xtask mod compile` checks that the scripts compile.
