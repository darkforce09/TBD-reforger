**Status:** live

# Mod UI screens

The in-game screens of the TBD Framework [mod](/documentation_v2/glossary.md#mod): where each
screen's code and layouts live, how the pre-game screens are built, and one folder per screen with
its specification and design references. Developers and agents read it before adding or moving a
screen, a panel or a layout.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/
├── admin_help_ticket/            the player-to-admin help ticket popup and the admin tickets panel
├── briefing/                     the Briefing screen over the map: navigation, ten pages, deploy
├── debrief_after_action_review/  the DEBRIEF scoreboard after the END banner
├── discord_identity_link/        linking a game identity to a platform account
├── end_screen/                   the END banner: winner and reason
├── in_game_menu/                 the pause menu's added actions and the admin menu
├── lobby/                        the Lobby screen: factions, squads and seats, kit inspector
├── mission_selection/            the Mission Selector: terrains, missions, mission inspector
├── objective_capture_hud/        the live objective board and capture bar
├── play_area_warning/            the out-of-bounds warning and countdown
├── safe_start_hud/               the safe start countdown and weapons-cold notices
├── spectator/                    the one-life spectator camera and roster
└── tactical_marker_palette/      placing side-scoped markers on the map
```

## How it works

Each screen folder holds a README index and `<screen>_specification.md`, a
[feature doc](/documentation_v2/standards/templates/feature_doc.md) of the screen as built and of
its design target. Nine of them also hold `visual_references/`, with its own README: the Stitch
mockup sets (`<panel>_mockup/`, an HTML export and its PNG) and, for the briefing, in-game menu,
lobby, mission selection and spectator screens, `reference_screenshots/`, the Arma 3 captures the
design started from, whose README breaks down each capture. The objective HUD, play area warning,
safe start HUD and tactical marker palette folders have no design references.

### Where a screen's code lives

The script side follows one rule: `apps/mod/tbd-framework/Scripts/Game/TBD/UI/` is a view layer
(the menu framework, shared widget handlers, the HUD and the mock catalogs) and holds nothing that
talks to the server; each feature module keeps its wire code beside its screen:

| File role | Runs on | Holds |
|---|---|---|
| `TBD_<X>Data.c` | both | plain model classes |
| `TBD_<X>Service.c` | server | builds payloads from the mission document; `Serialise` / `Parse` |
| `TBD_<X>Controller.c` | both | `modded class SCR_PlayerController` RPC pairs |
| `TBD_<X>Client.c` | client | static cache and the invokers screens subscribe to |
| `TBD_<X>Component.c` | server | the `SCR_BaseGameModeComponent` lifecycle host |
| `TBD_<X>Catalog.c` | client | the read surface a screen draws from |
| `UI/` | client | the screen class and its panels |

A screen reads its catalog and never a service or the mission document. The four pre-game
catalogs (`TBD_MissionCatalog`, `TBD_LobbyCatalog`, `TBD_BriefingCatalog`, `TBD_PlayersCatalog`)
build themselves from the mocks in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` until
something calls their `Set()`, and no script does, so those screens show sample data.

| Screen | Scripts | Layouts | Opened by |
|---|---|---|---|
| [Mission selection](/documentation_v2/mod/tbd-framework/UI/mission_selection/README.md) | [Session/MissionSelector/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/) | [Session/MissionSelector](/apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/) | `TBD_UIMissionSelector` preset: `LOBBY` stage, F9, the top-bar tab |
| [Lobby](/documentation_v2/mod/tbd-framework/UI/lobby/README.md) | [Session/Lobby/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) | [Session/Lobby](/apps/mod/tbd-framework/UI/layouts/Session/Lobby/) | `TBD_UILobby` preset: the top-bar tab, the pause menu's "Change slot" |
| [Briefing](/documentation_v2/mod/tbd-framework/UI/briefing/README.md) | [Session/Briefing/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) | [Session/Briefing](/apps/mod/tbd-framework/UI/layouts/Session/Briefing/) | `TBD_UIBriefing` preset: `BRIEFING` stage, the top-bar tab |
| In-game menu | [Session/Admin/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/); the pause hook in `TBD_LobbyScreen.c` | the list shell in [Common](/apps/mod/tbd-framework/UI/layouts/Common/) | `TBD_UIAdmin` preset (F8); the game's `PauseMenuUI` |
| Spectator | [Session/Spectator/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) | the list shell in [Common](/apps/mod/tbd-framework/UI/layouts/Common/) | `TBD_Spectator` preset, after death |
| End screen, debrief | [Session/PostGame/UI](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/UI/) | [Session/PostGame](/apps/mod/tbd-framework/UI/layouts/Session/PostGame/) | `END` and `DEBRIEF` stages, as workspace overlays |
| Objective HUD | [UI/Hud](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/) | [Hud](/apps/mod/tbd-framework/UI/layouts/Hud/) | the objective snapshot the server sends each client |
| Safe start, play area, identity link, markers | [Gamemode/Stages](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/), [Systems/Zones](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/), [API](/apps/mod/tbd-framework/Scripts/Game/TBD/API/), [Systems/Markers](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/) | none: engine pop-ups, chat and map markers | the stage, the zone check, a chat command, the mission |
| Admin help ticket | none | none | no code yet; the folder holds the design target |

### The pre-game dock shell

The Mission Selector, lobby and briefing are dock shells: a layout of empty named docks that a
`TBD_DockScreen` subclass (`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_DockScreen.c`)
fills at open. `TopDock` (56 high) and `BottomDock` (64 high) take the shared bars from
`apps/mod/tbd-framework/UI/layouts/Session/Shared/`; `LeftDock`, `CenterDock` and `RightDock` take
the screen's panels, at widths the shell sets; `OverlayDock`, full screen, last and hidden while
empty, hosts popovers. The briefing adds `WideDock` for the players panel and lays everything over
the live map. The top bar's tabs, "Scenario Browser", "Lobby" and "Briefing", move between the
three screens. The [Session layouts README](/apps/mod/tbd-framework/UI/layouts/Session/README.md)
draws the whole flow.

### Rules

- A shell holds only its backdrop or map and empty docks; a screen whose layout would grow large
  splits into a shell plus sub-layouts mounted into the docks.
- Every layout is named once, as a constant in `TBD_UILayouts`
  (`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`), and screens use the
  constant. `TBD_UILayouts.Create` falls back to the bare path when a GUID does not resolve, which
  does not replace a resource database pass.
- Layout GUIDs are `7BD1A7000000XXnn`: `XX` a block per layout, `nn` `00` the root widget, `01`
  the `.meta` resource id, `02` and up the children. The block ledger heads `TBD_UILayouts.c`;
  grep it before taking a block.
- Colour is a `TBD_UITheme` token or a `TBD_EUITint`, and icons are `TBD_UIIcons` keys; a layout
  carries placeholder colours only.
- Moving a layout is `git mv`, the `.meta` `Name`, the `TBD_UILayouts` constant and, for a menu
  shell, `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`; then
  [Workbench](/documentation_v2/glossary.md#workbench) opens `addon.gproj` and rewrites
  `resourceDatabase.rdb`, since non-script resources are not found by a directory scan.
- New `.c` files compile headless with `cargo xtask mod compile`; the
  [framework README](/apps/mod/tbd-framework/README.md) covers the Workbench restart they need.
- `apps/mod/tbd-export/` and `apps/mod/tbd-emcp/` hold no UI and no copy of this tree.

## Code

- [Framework interface assets](/apps/mod/tbd-framework/UI/) — the layouts and textures of every
  screen
- [UI scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/) — the menu stack, dock screen, theme,
  shared primitives, HUD and mock catalogs
- [Session scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/) — each screen's feature
  module and its `UI/` folder

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [style lock](/documentation_v2/refactor_style_lock.md) for the screen specifications; the
  code folders above.
- Used by: `TBD_UILayouts.c`, `TBD_MissionSelectorData.c` and `TBD_PlayersCatalog.c`, whose
  comments cite this index; the in-code READMEs of the screens, which link their specification;
  the [mod design](/documentation_v2/mod/tbd-framework/mod_design.md).
- Rules: one folder per screen, named in snake_case after the screen; a specification describes
  the built screen first and its design target second; mockup sets keep the `<panel>_mockup/`
  name and their HTML and PNG together.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the design methodology and
  the one-life rules every screen serves
- [Common layouts](/apps/mod/tbd-framework/UI/layouts/Common/README.md) — the shared primitives
  and their handler contracts
