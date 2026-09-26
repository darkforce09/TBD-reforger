# Lobby screen layouts

The layouts of the Lobby tab, where a player picks a faction and claims a
[slot](/documentation_v2/glossary/n_to_z.md#slot) in the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat): a
dock shell, the faction and roster column bodies, the squad cards and seat rows, and the kit
inspector with its 3D preview. `TBD_LobbyScreen` fills the shell from `TBD_LobbyCatalog`, which
serves mock data, so claims stay on the local client.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Lobby/
├── TBD_KitInspector.layout*      right column: title band with the seat line, scrolling cards
├── TBD_KitPreview.layout*        preview card: a 3D doll wearing the seat's kit, and a caption
├── TBD_KitWeaponCard.layout*     one weapon slot card: name, attachments and ammunition
├── TBD_LobbyFactionList.layout*  FACTIONS body: faction rows, a spectators row, an empty dock
├── TBD_LobbyFactionRow.layout*   one faction or spectators row with role and count chips
├── TBD_LobbyRoster.layout*       ROLES body: a clipped scrolling list of squad cards
├── TBD_LobbyScreen.layout*       the shell: backdrop and empty docks
├── TBD_LobbySlotRow.layout*      one seat: role, weapon and trait chips, holder, status chip
└── TBD_LobbySquadCard.layout*    one collapsible squad: header chips, Locate dock, seat rows
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## How it works

`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds the `TBD_UILobby` preset to
`TBD_LobbyScreen.layout` and the `TBD_LobbyScreen` class
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/TBD_LobbyScreen.c`), a `TBD_DockScreen`.
The shell holds a `Backdrop`, a `WindowFrame` inset 16 px, and the docks, in reference pixels:

| Dock | Geometry | Mounted there | By |
|---|---|---|---|
| `TopDock` | full width, 56 high | the session top bar, the [mission](/documentation_v2/glossary/g_to_m.md#mission) id as the title | `TBD_DockScreen` |
| `LeftDock` | x 0, 320 wide, y 68 to 76 above the bottom | `TBD_PanelFill` with the `TBD_LobbyFactionList` body | `TBD_LobbyFactionPanel` |
| `CenterDock` | x 332, 500 wide | `TBD_PanelFill` with the `TBD_LobbyRoster` body | `TBD_LobbyRosterPanel` |
| `RightDock` | x 844 to the right edge | `TBD_KitInspector` | `TBD_KitInspectorPanel` |
| `BottomDock` | full width, 64 high | the session bottom bar: Lock Lobby, Ready & Continue | `TBD_DockScreen` |
| `OverlayDock` | full screen, last child, hidden while empty | popovers | `TBD_DropdownComponent` |

The roster panel creates a `TBD_LobbySquadCard` per squad and a `TBD_LobbySlotRow` per seat; the
kit inspector stacks `TBD_Panel` cards from `apps/mod/tbd-framework/UI/layouts/Common/` in
`CardsContent`, with a `TBD_KitPreview` card, `TBD_KitWeaponCard`s in `TBD_Columns3` or
`TBD_Columns2` rows, and grids of `TBD_StatCell`s in `TBD_Columns2` to `TBD_Columns4` rows. The
briefing's ORBAT page reuses `TBD_KitInspector` and the roster column, read only.

| Layout | Handler | Widgets the handler binds |
|---|---|---|
| `TBD_LobbyFactionList` | `TBD_LobbyFactionPanel` | `Stack`, `Content`, `SpectatorDock`, `VoiceDock` |
| `TBD_LobbyFactionRow` | `TBD_LobbyFactionRowComponent` | `Border`, `Background`, `Name`, `RoleChipDock`, `CountChipDock` |
| `TBD_LobbyRoster` | `TBD_LobbyRosterPanel` | `ListFrame`, `Scroll`, `Content`, `ScrollBarDock`, `EmptyState` |
| `TBD_LobbySquadCard` | `TBD_LobbySquadCardComponent` | `Border`, `Background`, `HeaderButton`, `HeaderOverlay`, `HeaderBG`, `CallsignChipDock`, `VehicleChipDock`, `ActionDock`, `CountChipDock`, `Chevron`, `HeaderRule`, `SlotsContent` |
| `TBD_LobbySlotRow` | `TBD_LobbySlotRowComponent` | `Background`, `RoleText`, `ChipsDock`, `HolderText`, `StatusChipDock`, `RowRule` |
| `TBD_KitInspector` | `TBD_KitInspectorPanel` | `PanelBorder`, `PanelBG`, `Header`, `HeaderBG`, `HeaderIcon`, `Title`, `SlotRow`, `SlotTitle`, `SlotChipsDock`, `HeaderRule`, `BodyFrame`, `Scroll`, `CardsContent`, `ScrollBarDock`, `EmptyState` |
| `TBD_KitPreview` | `TBD_KitInspectorPanel`; `TBD_KitPreviewComponent.Attach` on `Preview` | `CardBorder`, `CardBG`, `Box`, `Border`, `Background`, `GridClip`, `GridImage`, `Preview`, `Label` |
| `TBD_KitWeaponCard` | `TBD_KitInspectorPanel` | `Border`, `Background`, `SlotChipDock`, `NameBorder`, `NameBG`, `Name`, `AttachmentsTitle`, `AttachmentsContent`, `AmmoRule`, `AmmoTitle`, `AmmoSummary`, `AmmoContent` |

`VoiceDock` is an empty frame: no script mounts anything into it. The seat rows use a square image
`Background`, since they sit inside a rounded card; the squad header's fill is taller than its
clipping `HeaderOverlay`, so only its top corners are round. The lists follow the scroll-clip
recipe of `TBD_ScrollList`, with a `TBD_UIScrollBar` in each `ScrollBarDock`.

## Format

- File type: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) widget layouts (`.layout`), plain
  text, each beside a `.layout.meta` whose `Name` holds
  `{GUID}UI/layouts/Session/Lobby/<file>.layout`. Every `*Border`, `*BG` and card `Background` is
  an empty `FrameWidgetClass` dock that the handler fills with a rounded shape (columns and the kit
  inspector 12, rows and cards 8, chips and cells 6); every text widget carries a `FontProperties`
  block; `Preview` is an `ItemPreviewWidgetClass`.
- Resource GUID: `7BD1A7000000XX01` in each `.meta`, from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`: the shell `0C`, then
  `TBD_LobbyFactionList` `2F`, `TBD_LobbyFactionRow` `30`, `TBD_LobbyRoster` `31`,
  `TBD_LobbySquadCard` `32`, `TBD_LobbySlotRow` `33`, `TBD_KitInspector` `34`, `TBD_KitPreview`
  `35` and `TBD_KitWeaponCard` `37`. The shell's GUID is named by the menu config and never changes.
- Naming: `TBD_Lobby<Element>.layout` for the roster and factions, `TBD_Kit<Element>.layout` for
  the kit inspector; a layout another screen needs moves to `Common/`.
- Adding a layout: take a free block from the ledger, author the layout and its `.meta`, add a
  `LOBBY_*` constant to `TBD_UILayouts`, and commit both files; the game finds a new path only
  after [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) has rewritten `resourceDatabase.rdb`.

## Referenced by

- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` names the shell by GUID in the
  `TBD_UILobby` preset.
- `TBD_UILayouts` names each layout by GUID and path (`LOBBY_SCREEN` and the `LOBBY_*` constants);
  the panels in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` use them, as the
  handler table shows.
- `TBD_BriefingOrbatPage` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/TBD_BriefingPageComms.c` mounts
  `LOBBY_KIT_INSPECTOR` and, through `TBD_LobbyRosterPanel`, the roster layouts.

## Boundaries

- Depends on: the layouts in `apps/mod/tbd-framework/UI/layouts/Common/` and the session bars in
  `apps/mod/tbd-framework/UI/layouts/Session/Shared/`; the handler classes named above; the
  engine's `ItemPreviewWidgetClass` for the doll.
- Used by: the lobby screen and, read only, the briefing's ORBAT page.
- Rules: the widget names above are the handlers' contract; the shell keeps its GUID and path,
  since the menu config names them; a layout and its `.meta` are committed together.

## Related documentation

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md)
  — the screen as built, its
  data, design target, open work and decisions
- [Lobby design references](/documentation_v2/mod/tbd-framework/UI/lobby/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
