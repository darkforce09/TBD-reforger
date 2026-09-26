# Session screen layouts

The layouts of the screens a player meets across a session: the pre-game Mission Selector, lobby
and briefing, the post-game END and DEBRIEF overlays, and the bars and panels those screens share.
Each folder but `Pause/` and `Shared/` is named as a script folder under
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/`, whose `UI/` classes drive its layouts.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/
├── Admin/            none: the admin menu draws on the shared list shell
├── Briefing/         the briefing shell over the map, its navigation panels and page parts
├── Lobby/            the lobby shell, faction and roster columns, squad cards, kit inspector
├── MissionSelector/  the Mission Selector shell, terrain and mission columns, inspector
├── Pause/            none: the pause menu is the game's own, with one added button
├── PostGame/         the END banner and DEBRIEF scoreboard overlays
├── Shared/           the session top and bottom bars and the players panel
└── Spectator/        none: the spectator roster draws on the shared list shell
```

## How it works

The three pre-game screens are dock shells: a layout of empty named docks that a
`TBD_DockScreen` subclass fills at open. `TBD_DockScreen`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_DockScreen.c`) mounts the `Shared/` bars
into `TopDock` and `BottomDock`, the screen mounts its columns into `LeftDock`, `CenterDock` and
`RightDock`, and dropdown menus open in `OverlayDock`, the full-screen last child that stays hidden
while empty. Column widths belong to each shell. The top bar's tabs move between the three
screens.

```text
Mission Selector  <── top-bar tabs ──>  Lobby  <── top-bar tabs ──>  Briefing (over the map)
      │                                   │                              │
      └──────────── Shared/ top bar, bottom bar ─────────────────────────┘
                                                        Players mode ─> Shared/ players panel
END stage   ─> PostGame/TBD_EndScreen.layout      (workspace overlay, not a menu)
DEBRIEF     ─> PostGame/TBD_DebriefScreen.layout  (workspace overlay, not a menu)
```

The admin and spectator menus open the list shell
`apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout` through their presets in
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`, so `Admin/` and `Spectator/` hold no
layouts; `Pause/` holds none because the pause menu is the game's `PauseMenuUI`. The Mission
Selector, lobby and briefing read mock catalogs from
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`, so their layouts show sample data.

## Format

- File type: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) widget layouts (`.layout`), each
  beside a `.layout.meta` that holds its resource GUID; each child README lists its widget names.
- Resource GUID: `7BD1A7000000XX01`, one block per layout from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`. A shell a menu preset names
  keeps its GUID and path.
- Naming: one folder per screen; a layout two screens share goes in
  `Shared/`, and a primitive any screen may use in `apps/mod/tbd-framework/UI/layouts/Common/`.
- Adding a layout: as each child README describes, with a `TBD_UILayouts` constant and a
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) pass that rewrites `resourceDatabase.rdb`.

## Referenced by

- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` names the three pre-game shells by GUID
  in the `TBD_UIMissionSelector`, `TBD_UILobby` and `TBD_UIBriefing` presets.
- `TBD_UILayouts` names every layout by GUID and path; the screen classes under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/` and the bar handlers in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/` use its constants.

## Boundaries

- Depends on: the primitives in `apps/mod/tbd-framework/UI/layouts/Common/`, the textures in
  `apps/mod/tbd-framework/UI/Textures/TBD/`, the vanilla map layout under the briefing, and the
  handler classes the layouts name.
- Used by: the session screens in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Rules: a shell holds its backdrop or map and empty named docks, which the screen class fills;
  every layout has a `TBD_UILayouts` constant and nothing instantiates a bare path; a layout and
  its `.meta` are committed together.

## Related documentation

- [Mod UI structure](/documentation_v2/mod/tbd-framework/UI/README.md)
  — where each UI file goes and which mockup panel lands in which folder
