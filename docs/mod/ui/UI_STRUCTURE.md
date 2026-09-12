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
  Core/            TBD_Log  TBD_Registry  TBD_RegistryPocComponent
  API/             TBD_BackendConfig  TBD_ResultsReporter  TBD_IdentityLink  TBD_PlayerIdentity
  Gamemode/        Orchestrator/ (TBD_FrameworkManager) · Stages/ (GameStage, Safestart, WinCondition) · Objectives/
  Systems/         Mission/ (Data, Ingestion, Loaders) · Spawning/ · Loadouts/ · Audio/ · Zones/ · Radio/ · Markers/ · AI/
  Session/         Lobby/ (+PreSlot, +UI) · Briefing/ (+UI) · Spectator/ (+UI) · Admin/ (+UI) · MissionSelector/ (+Browser, +Router, +UI) · PostGame/ (+UI)
  UI/              Core/ (UILayouts, UITheme, MenuBase, MenuStack, ShellScreen, UIButton, ListBox)
                   Common/ (shared row/chip handlers) · Hud/ (ObjectiveHud, TaskHud) · Mock/
```

## Layouts

`UI/layouts/` mirrors the 7-domain architecture. Every `.layout` has a sibling `.meta` whose `Name` is `"{GUID}UI/layouts/<same path>"`.

```
UI/layouts/
  Common/          shared component library (TBD_ScreenShell, TBD_ListRow, future Button, Card)
  Hud/             TBD_ObjectiveHud
  Session/
    Shared/        pregame bottom bar, voice panel, players modal
    MissionSelector/ TBD_MissionSelector, TBD_TerrainRow, TBD_MissionCard
    Lobby/         TBD_LobbyScreen (+9 docks: Header, Footer, Sidebar, Roster, Inspector, SquadCard...)
    Briefing/      TBD_BriefingScreen (+Panels)
    Spectator/     top/bottom bar, roster, combat_details
    Admin/         admin_panel_sidebar + 10 panels
    Pause/         pause_menu_left_sidebar, player_options, staging_phase, identity_link
    PostGame/      TBD_EndScreen, TBD_DebriefScreen
```

Folders that are empty today carry a `README.md` naming the mockup panels that land there.

## Where does a mockup panel go?

| Mockup group / panel | Layout folder | Script folder |
|---|---|---|
| pregame · mission_selector_top_bar, lobby_bottom_bar, voice_panel, players_panel | `Session/Shared/` | `UI/Common/` |
| pregame · terrain_selector, scenario_browser, mission_inspector | `Session/MissionSelector/` | `Session/MissionSelector/UI/` |
| pregame · lobby_sidebar, orbat_panel, slot_kit_inspector | `Session/Lobby/` | `Session/Lobby/UI/` |
| pregame · primary_navigation, briefing_navigation, frequencies, objectives, rules, lore, parameters, markers, friendly/enemy assets, uniforms | `Session/Briefing/` (+`Panels/`) | `Session/Briefing/UI/` |
| ingame_menu · pause_menu_left_sidebar, player_options, staging_phase, identity_link | `Session/Pause/` | `Session/Pause/UI/` |
| ingame_menu · admin_panel_sidebar + 10 admin panels | `Session/Admin/` (+`Panels/`) | `Session/Admin/UI/` |
| ingame_hud · spectator top/bottom bar, roster, combat_details | `Session/Spectator/` | `Session/Spectator/UI/` |
| postgame · end_screen_banner, aar | `Session/PostGame/` | `Session/PostGame/UI/` |
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
- `apps/mod/tbd-export/` is a thin addon that **depends on** `tbd-framework` (map-export tooling only —
  no UI of its own, nothing to mirror). `apps/mod/tbd-emcp/` carries the enfusion-mcp Workbench
  bridge handlers. Neither holds a copy of this UI tree.
