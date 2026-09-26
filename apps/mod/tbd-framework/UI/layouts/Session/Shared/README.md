# Shared session screen layouts

The layouts more than one session screen wears unchanged: the top and bottom bars of the
Mission Selector, lobby and briefing, and the PLAYERS panel with its lanes and rows. The bars'
handlers live in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`, and
`TBD_DockScreen` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_DockScreen.c` mounts them.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Shared/
├── TBD_PlayerLane.layout*        one lane of the players panel: tinted header, columns, list dock
├── TBD_PlayerRow.layout*         one player row: index, icon, name, tag chip, ping
├── TBD_PlayersPanel.layout*      the PLAYERS window: title, total and four lane docks
├── TBD_SessionBottomBar.layout*  the bottom bar: left and right action rows the screen fills
└── TBD_SessionTopBar.layout*     the top bar: title, tab strip, identity box and player count
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## How it works

Every `TBD_DockScreen` shell declares a 56 px `TopDock` and a 64 px `BottomDock`. On open,
`TBD_DockScreen` mounts `TBD_SessionTopBar` and `TBD_SessionBottomBar` there, unless the screen
turns them off (`UsesTopBar`, `UsesBottomBar`), and wires them: the title and tab from the screen,
the identity and player count when the screen supplies them, and the screen's own actions added to
the bottom bar with `AddAction`. The bar holds no buttons of its own, so the selector's
Select Scenario and the lobby's Lock Lobby and Ready & Continue are rows of `TBD_Button` from
`apps/mod/tbd-framework/UI/layouts/Common/` in one layout. The top bar's tabs are a
`TBD_TabStrip` mounted into `TabStripDock`.

The players panel is not a menu: `TBD_PlayersPanel`
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/TBD_PlayersPanel.c`) builds
`TBD_PlayersPanel.layout` into the briefing's `WideDock` and removes it on the next mode, since a
menu pushed over the briefing would close its map. The window fills the dock; `BluforDock` and
`OpforDock` split the top 63 % side by side, and `SpectatorDock` and `UnslottedDock` the rest, all
anchored so they resize with the dock. Each dock takes a `TBD_PlayerLane`, whose `ListDock` holds a
`TBD_ScrollList` of `TBD_PlayerRow`s; the lane inks each ping green under 40 ms, amber under
80 ms and red above.

| Layout | Handler | Widgets the handler binds |
|---|---|---|
| `TBD_SessionTopBar` | `TBD_SessionTopBar` | `BarBorder`, `BarBG`, `Title`, `TabStripDock`, `IdentityBox`, `IdentityBorder`, `IdentityBG`, `IdentityName`, `IdentityRoleDock`, `CountBox`, `CountBorder`, `CountBG`, `CountIcon`, `CountText` |
| `TBD_SessionBottomBar` | `TBD_SessionBottomBar` | `BarBorder`, `BarBG`, `LeftActions`, `RightActions` |
| `TBD_PlayersPanel` | `TBD_PlayersPanel` | `Window`, `WindowBorder`, `WindowBG`, `Header`, `Title`, `TotalText`, `HeaderRule`, `BluforDock`, `OpforDock`, `SpectatorDock`, `UnslottedDock` |
| `TBD_PlayerLane` | `TBD_PlayerLane` | `Border`, `Background`, `HeaderClip`, `HeaderBG`, `NameText`, `RoleChipDock`, `CountLabel`, `CountChipDock`, `ColIndex`, `ColPlayer`, `ColPing`, `ColumnRule`, `ListDock` |
| `TBD_PlayerRow` | written by `TBD_PlayerLane` | `Index`, `Icon`, `Name`, `TagChipDock`, `Ping`, `RowRule` |

## Format

- File type: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) widget layouts (`.layout`), plain
  text, each beside a `.layout.meta` whose `Name` holds
  `{GUID}UI/layouts/Session/Shared/<file>.layout`. The bars carry their handler as a component on
  the root; every `*Border` and `*BG` is an empty `FrameWidgetClass` dock that the handler fills
  with a rounded shape, and every text widget carries a `FontProperties` block.
- Resource GUID: `7BD1A7000000XX01` in each `.meta`, from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`: `TBD_SessionTopBar` `1B`,
  `TBD_SessionBottomBar` `1C`, `TBD_PlayersPanel` `43`, `TBD_PlayerLane` `44` and `TBD_PlayerRow`
  `45`; a GUID never changes.
- Naming: `TBD_<Element>.layout`. A layout belongs here when two or more session screens use it,
  or when any screen may host it, as with the players panel.
- Adding a layout: take a free block from the ledger, author the layout and its `.meta`, add a
  `TBD_UILayouts` constant, and commit both files; the game finds a new path only after
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) has rewritten `resourceDatabase.rdb`.

## Referenced by

- `TBD_UILayouts` names each layout by GUID and path: `SESSION_TOP_BAR`, `SESSION_BOTTOM_BAR`,
  `PLAYERS_PANEL`, `PLAYERS_LANE` and `PLAYERS_ROW`.
- `TBD_DockScreen.OnScreenOpen` mounts the two bars for its subclasses: `TBD_MissionSelectorScreen`,
  `TBD_LobbyScreen` and `TBD_BriefingScreen` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.
- `TBD_PlayersPanel` and `TBD_PlayerLane` create the panel, lanes and rows; `TBD_BriefingScreen`
  builds the panel in its Players mode.

## Boundaries

- Depends on: `TBD_TabStrip`, `TBD_Button`, `TBD_Chip` and `TBD_ScrollList` from
  `apps/mod/tbd-framework/UI/layouts/Common/`; the handler classes named above.
- Used by: the Mission Selector, lobby and briefing screens, and the briefing's Players mode.
- Rules: the widget names above are the handlers' contract; the bars hold no screen-specific
  content, which the screen supplies at run time; a layout and its `.meta` are committed together.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the briefing's design target, the players panel included
- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md)
  — the lobby's design target, its bottom bar included
