# Mod UI structure (`apps/mod/tbd-framework`)

Set 2026-09-12, before the Stitch-mockup rebuild. Source of truth for **where a UI file goes**.
Mockups: [`ui_stitch_mockup/`](ui_stitch_mockup/) (4 groups, 43 panels). Layout path registry:
`Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`.

## Principle

`Scripts/Game/TBD/UI/` is a **view layer**. It holds screens, widget handlers, the menu framework and mock
data - nothing that talks to the server. Wire code lives in **feature modules** next to `Radio/` and
`Markers/`, which already follow this shape:

| File role | Runs on | Holds |
|---|---|---|
| `TBD_XData.c` | both | plain model classes |
| `TBD_XService.c` | server | builds payloads from the mission doc; `Serialise` / `Parse` |
| `TBD_XController.c` | both | `modded class SCR_PlayerController` RPC pairs |
| `TBD_XClient.c` | client | static cache + `ScriptInvoker`s the screens subscribe to |
| `TBD_XComponent.c` | server | `SCR_BaseGameModeComponent` lifecycle host |
| `TBD_XStage.c` | client | stage watcher that raises / drops a screen (Lobby only, so far) |

A screen reads `TBD_XClient` (or `UI/Mock/` while unwired) and never a Service or the mission document.
Shared player data, when it arrives, becomes a sibling module `Players/`.

## Scripts

```
Scripts/Game/TBD/
  UI/
    Core/          TBD_UILayouts  TBD_UITheme  TBD_MenuBase  TBD_MenuStack  TBD_ShellScreen
                   TBD_UIInteractive  TBD_UIButton  TBD_ListBox  TBD_ListBoxRow
    Common/        shared widget handlers (NavItem, FactionChip, PlayerRow, SquadCard, SearchBar, …)
    Mock/          TBD_LobbyMockData  TBD_MissionSelectorData   (all mock data, cross-screen)
    PreGame/       MissionSelector/  Lobby/ (+TBD_LoadoutPreview)  Briefing/
    InGameMenu/    Admin/ (TBD_AdminScreen)  Pause/
    Hud/           TBD_ObjectiveHud  TBD_TaskHud  Spectator/ (TBD_SpectatorScreen)
    PostGame/      TBD_EndScreen  TBD_DebriefScreen
  Lobby/           Data  Service  Controller  Client  Stage  Component
  Briefing/        Data  Service  Controller  Client
  Admin/           Data  SnapshotService  Client  Audit  Commands  Service
  Spectator/       camera / host / targets (no screen)
```

## Layouts

`UI/layouts/` mirrors the script tree. Each screen folder holds its screen layout plus its row / card
sub-layouts. Every `.layout` has a sibling `.meta` whose `Name` is `"{GUID}UI/layouts/<same path>"`.

```
UI/layouts/
  Core/            TBD_ScreenShell  TBD_ListRow
  Common/          shared sub-layouts
  PreGame/         Shared/  MissionSelector/  Lobby/  Briefing/ (+ Panels/ when built)
  InGameMenu/      Pause/  Admin/ (+ Panels/)
  Hud/             TBD_ObjectiveHud  Spectator/
  PostGame/        TBD_EndScreen  TBD_DebriefScreen
```

Folders that are empty today carry a `README.md` naming the mockup panels that land there.

## Where does a mockup panel go?

| Mockup group / panel | Layout folder | Script folder |
|---|---|---|
| pregame · mission_selector_top_bar, lobby_bottom_bar, voice_panel, players_panel | `PreGame/Shared/` | `UI/PreGame/Shared/` |
| pregame · terrain_selector, scenario_browser, mission_inspector | `PreGame/MissionSelector/` | `UI/PreGame/MissionSelector/` |
| pregame · lobby_sidebar, orbat_panel, slot_kit_inspector | `PreGame/Lobby/` | `UI/PreGame/Lobby/` |
| pregame · primary_navigation, briefing_navigation, frequencies, objectives, rules, lore, parameters, markers, friendly/enemy assets, uniforms | `PreGame/Briefing/` (+`Panels/`) | `UI/PreGame/Briefing/` |
| ingame_menu · pause_menu_left_sidebar, player_options, staging_phase, identity_link | `InGameMenu/Pause/` | `UI/InGameMenu/Pause/` |
| ingame_menu · admin_panel_sidebar + 10 admin panels | `InGameMenu/Admin/` (+`Panels/`) | `UI/InGameMenu/Admin/` |
| ingame_hud · spectator top/bottom bar, roster, combat_details | `Hud/Spectator/` | `UI/Hud/Spectator/` |
| postgame · end_screen_banner, aar | `PostGame/` | `UI/PostGame/` |
| any row / chip / chrome reused by 2+ screens | `Common/` | `UI/Common/` |

## Rules

- No `.layout` over 1000 lines: split into a screen shell + sub-layouts injected into named docks
  (see `TBD_LobbyScreen.layout` + `TBD_LobbyScreenComponent`). Existing oversize files
  (`TBD_MissionSelector` 2736, `TBD_LobbyInspector` 1312) are split when that screen is rebuilt.
- Every layout path is named once, in `TBD_UILayouts`; screens use the constant. No bare-path fallbacks.
- GUIDs are `7BD1A7000000XXnn` (`XX` = block per screen, `nn` = `00` root, `01` meta id, `02+` children).
  Blocks in use / reserved are listed in the `TBD_UILayouts.c` header. Grep before taking one.
- Moving a layout = `git mv` + edit the `.meta` `Name` + the `TBD_UILayouts` constant +
  `Configs/System/chimeraMenus.conf` if it is a menu preset. Then a Workbench open of `addon.gproj`
  to rewrite `resourceDatabase.rdb` (non-script resources are not directory-scanned).
- New `.c` files: Workbench cold restart (see `tbd-framework/README.md`). Headless compile gate:
  `cargo xtask mod compile` (`hcargo` from inside the container).
- `apps/mod/tbd-export/` is a separate mod with a stale copy of the pre-reorg UI. Not mirrored.
