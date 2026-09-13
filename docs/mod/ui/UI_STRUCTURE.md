# Mod UI structure (`apps/mod/tbd-framework`)

Set 2026-09-12, before the Stitch-mockup rebuild; updated the same day when the first rebuilt
screen (Mission Selector) shipped. Source of truth for **where a UI file goes**.
Mockups: [`ui_stitch_mockup/`](ui_stitch_mockup/) (4 groups, 43 panels). Layout path registry:
`Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`. Component contracts:
`apps/mod/tbd-framework/UI/layouts/Common/README.md`.

## Principle

`Scripts/Game/TBD/UI/` is a **view layer**. It holds screens, widget handlers, the menu framework and mock
data - nothing that talks to the server. Wire code lives in **feature modules** next to `Radio/` and
`Markers/`, which already follow this shape:

| File role | Runs on | Holds |
|---|---|---|
| `TBD_XData.c` | both | plain model classes (Mission Selector: `TBD_MissionSelectorData.c` incl. the `TBD_MissionCatalog` read surface) |
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
  Session/         Lobby/ (+PreSlot, +UI) · Briefing/ (+UI) · Spectator/ (+UI) · Admin/ (+UI) · MissionSelector/ (+Browser, +Router, +Data, +UI) · PostGame/ (+UI)
  UI/              Core/ (UILayouts, UITheme, UIIcons, MenuBase, MenuStack, DockScreen, ShellScreen, UIButton, UIInteractive, ListBox)
                   Common/ (Panel, Chip, SearchBox, TabStrip, KeyValueRow, Dropdown, SessionTopBar, SessionBottomBar) · Hud/ (ObjectiveHud, TaskHud) · Mock/
```

## Layouts

`UI/layouts/` mirrors the 7-domain architecture. Every `.layout` has a sibling `.meta` whose `Name` is `"{GUID}UI/layouts/<same path>"`.

```
UI/layouts/
  Common/          component library: TBD_ScreenShell, TBD_ListRow, TBD_Panel, TBD_Chip, TBD_SearchBox,
                   TBD_NavItem + TBD_TabStrip, TBD_Button, TBD_KeyValueRow, TBD_Dropdown + TBD_DropdownMenu,
                   TBD_InsetText, TBD_Columns2
  Hud/             TBD_ObjectiveHud
  Session/
    Shared/        TBD_SessionTopBar, TBD_SessionBottomBar (shipped) · voice panel, players modal (pending)
    MissionSelector/ TBD_MissionSelector (dock shell) + TBD_TerrainSelector, TBD_TerrainRow, TBD_ScenarioBrowser,
                   TBD_MissionCard, TBD_MissionInspector, TBD_ModGridItem, TBD_FactionColumn
    Lobby/         TBD_LobbyScreen (dock shell) + TBD_LobbyFactionList, TBD_LobbyFactionRow, TBD_LobbyRoster,
                   TBD_LobbySquadCard, TBD_LobbySlotRow, TBD_KitInspector, TBD_KitPreview, TBD_KitCell, TBD_KitWeaponCard
    Briefing/      TBD_BriefingScreen (+Panels)
    Spectator/     top/bottom bar, roster, combat_details
    Admin/         admin_panel_sidebar + 10 panels
    Pause/         pause_menu_left_sidebar, player_options, staging_phase, identity_link
    PostGame/      TBD_EndScreen, TBD_DebriefScreen
```

Folders that are empty today carry a `README.md` naming the mockup panels that land there.

## The dock shell

A rebuilt screen is a shell (< 200 lines) with named docks, driven by a `TBD_DockScreen` subclass:
`TopDock` (56) and `BottomDock` (64) take the shared bars from `Session/Shared/`; `LeftDock`,
`CenterDock`, `RightDock` take the screen's panels; `OverlayDock` (full-bleed, last, hidden while
empty) hosts popovers. Column widths are the shell's own. Reference:
`UI/layouts/Session/MissionSelector/TBD_MissionSelector.layout` +
`Scripts/Game/TBD/Session/MissionSelector/UI/TBD_MissionSelectorScreen.c`.

## Where does a mockup panel go?

| Mockup group / panel | Layout folder | Script folder | Status |
|---|---|---|---|
| pregame · mission_selector_top_bar, lobby_bottom_bar | `Session/Shared/` | `UI/Common/` | **shipped** as `TBD_SessionTopBar` / `TBD_SessionBottomBar` |
| pregame · voice_panel, players_panel | `Session/Shared/` | `UI/Common/` | pending |
| pregame · terrain_selector, scenario_browser, mission_inspector | `Session/MissionSelector/` | `Session/MissionSelector/UI/` | **shipped** |
| pregame · lobby_sidebar, orbat_panel, slot_kit_inspector | `Session/Lobby/` | `Session/Lobby/UI/` | **shipped** (2026-09-13); kit visual preview pending |
| pregame · primary_navigation, briefing_navigation, frequencies, objectives, rules, lore, parameters, markers, friendly/enemy assets, uniforms | `Session/Briefing/` (+`Panels/`) | `Session/Briefing/UI/` | current screen; nav = vertical `TBD_TabStrip`, panels = `TBD_Panel` + `TBD_KeyValueRow` / `TBD_InsetText` |
| ingame_menu · pause_menu_left_sidebar, player_options, staging_phase, identity_link | `Session/Pause/` | `Session/Pause/UI/` | pending |
| ingame_menu · admin_panel_sidebar + 10 admin panels | `Session/Admin/` (+`Panels/`) | `Session/Admin/UI/` | pending |
| ingame_hud · spectator top/bottom bar, roster, combat_details | `Session/Spectator/` | `Session/Spectator/UI/` | pending |
| postgame · end_screen_banner, aar | `Session/PostGame/` | `Session/PostGame/UI/` | current screens |
| any row / chip / chrome reused by 2+ screens | `Common/` | `UI/Common/` | see the Common README table |

## Rules

- No `.layout` over 1000 lines: split into a screen shell + sub-layouts injected into named docks
  (see `TBD_MissionSelector.layout` + `TBD_MissionSelectorScreen`, or `TBD_LobbyScreen.layout` +
  `TBD_LobbyScreen`). No oversize layout remains since the lobby rebuild (2026-09-13).
- Every layout path is named once, in `TBD_UILayouts`; screens use the constant. No bare-path fallbacks.
- GUIDs are `7BD1A7000000XXnn` (`XX` = block per layout, `nn` = `00` root, `01` meta id, `02+` children).
  The block ledger is the header of `TBD_UILayouts.c`. Grep before taking one.
- Colour is a `TBD_UITheme` token or a `TBD_EUITint`; icons are `TBD_UIIcons` keys. A `.layout`
  carries placeholder colours only.
- Moving a layout = `git mv` + edit the `.meta` `Name` + the `TBD_UILayouts` constant +
  `Configs/System/chimeraMenus.conf` if it is a menu preset. Then a Workbench open of `addon.gproj`
  to rewrite `resourceDatabase.rdb` (non-script resources are not directory-scanned).
- New `.c` files: Workbench cold restart (see `tbd-framework/README.md`). Headless compile gate:
  `cargo xtask mod compile` (`hcargo` from inside the container).
- `apps/mod/tbd-export/` is a thin addon that **depends on** `tbd-framework` (map-export tooling only —
  no UI of its own, nothing to mirror). `apps/mod/tbd-emcp/` carries the enfusion-mcp Workbench
  bridge handlers. Neither holds a copy of this UI tree.
