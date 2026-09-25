# Players panel

The PLAYERS panel of the briefing: who is connected, in four lanes (BLUFOR, OPFOR, Spectators and
Unslotted), popped out beside the briefing's primary navigation while the map stays live behind it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/
└── TBD_PlayersPanel.c  `TBD_PlayersPanel` and its `TBD_PlayerLane` rows, built into a host dock
```

## How it works

`TBD_PlayersPanel` is a plain `Managed` controller, not a menu: `Build(dock)` instantiates
`TBD_UILayouts.PLAYERS_PANEL` (`apps/mod/tbd-framework/UI/layouts/Session/Shared/TBD_PlayersPanel.layout`)
into the host's dock and `Destroy()` removes it. It needs no scrim or window of its own, because a
menu pushed on top of the briefing would hide it and close its `SCR_MapEntity`. The header shows the
title and the total; each `TBD_PlayerLane` has a tinted header with a count, a column header and
scrolling rows of name, tag and ping, read from `TBD_PlayersCatalog.Get()`. Faction lanes take the
side tint; Spectators and Unslotted stay neutral.

## Authority

- Server: nothing.
- Client: everything; the panel is built on the local player's briefing screen.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_PlayersCatalog` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/`;
  `TBD_UILayouts` and `TBD_UITheme` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: `TBD_BriefingScreen` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`,
  which builds the panel into its `WideDock` in the PLAYERS mode and destroys it on the next mode.
- Rules: the panel never opens as a stacked menu over the briefing; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the briefing's design target, the players panel included
