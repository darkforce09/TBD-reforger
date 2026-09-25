# Spectator roster screen

The unit list a dead player sees while spectating: who is still alive, by faction and group, with
one click to watch any of them. It opens over the live spectator camera and never blocks it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/
└── TBD_SpectatorScreen.c  `TBD_SpectatorScreen`: the roster over the shared shell, and its preset
```

## How it works

`TBD_SpectatorScreen` extends `TBD_ShellScreen` and reuses the shared shell layout
(`apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`) through the `TBD_Spectator`
preset that the file's `modded enum ChimeraMenuPreset` adds and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds. On open it repaints the backdrop
transparent and the panel to `SURFACE_GLASS`, so the world stays visible behind the list. Every
`REFRESH_MS` (1 s) it asks `TBD_SpectatorTargets.Collect` for the watchable players and marks the
one `TBD_SpectatorController` follows. Clicking a player follows them; clicking the followed player
again toggles first person; the one button, FREE CAMERA, returns to free flight. The subtitle says
whether the view is limited to the viewer's own side, and the status line explains an empty list
(no faction resolved, nobody in view, nobody alive).

## Authority

- Server: nothing.
- Client: everything; `TBD_SpectatorController` opens and closes the screen through
  `TBD_MenuStack` on the spectating player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpectatorTargets` and `TBD_SpectatorController` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/`; `TBD_ShellScreen`, `TBD_UITheme` and
  `TBD_ListBox` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: `TBD_SpectatorController`, which opens the `TBD_Spectator` preset; the preset entry in
  `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`.
- Rules: the screen never pauses or captures the camera; the list shows only what
  `TBD_SpectatorTargets` allows; a new preset takes effect only after Workbench registers
  `chimeraMenus.conf`; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts
  compile.

## Related documentation

- [Spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
  — the spectator interface's design target
