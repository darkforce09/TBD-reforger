# UI/layouts/Session/Lobby

The Lobby (rebuilt 2026-09-13 from the `lobby_sidebar`, `orbat_panel_blufor` and
`slot_kit_inspector` Stitch mockups): a Dock & Sub-Layout screen driven by
`Scripts/Game/TBD/Session/Lobby/UI/TBD_LobbyScreen.c` (`TBD_DockScreen`). Data comes from
`TBD_LobbyCatalog.Get()` (mock today; a `TBD_LobbyClient` adapter later).

## Shell — `TBD_LobbyScreen.layout` (GUID `{7BD1A70000000C01}`, block `0C`)

Same shape as the selector shell; same GUID + path as the retired monolith so
`Configs/System/chimeraMenus.conf` and the rdb row stay valid.

| Dock | Geometry (reference px) | Mounted layout | Driven by |
|---|---|---|---|
| `TopDock` | full width, 56 high | `Session/Shared/TBD_SessionTopBar` (mono mission id, Lobby tab active) | `TBD_DockScreen` |
| `LeftDock` | x 0, width 320, y 68 → bottom-76 | `Common/TBD_PanelFill` + `TBD_LobbyFactionList` body | `TBD_LobbyFactionPanel` |
| `CenterDock` | x 332, width 500 | `Common/TBD_PanelFill` + `TBD_LobbyRoster` body | `TBD_LobbyRosterPanel` |
| `RightDock` | x 844 → right edge | `TBD_KitInspector` | `TBD_KitInspectorPanel` |
| `BottomDock` | full width, 64 high | `Session/Shared/TBD_SessionBottomBar` (`Lock Lobby` quiet, `Ready & Continue` primary) | `TBD_DockScreen` |
| `OverlayDock` | full-bleed, last child, hidden while empty | popovers | `TBD_DropdownComponent` |

## Sub-layouts

| Layout | GUID block | Role | Widget contract |
|---|---|---|---|
| `TBD_LobbyFactionList.layout` | `2F` | body of the FACTIONS panel | `Stack`, `Content` (faction rows), `SpectatorDock` (the Spectators row), `VoiceDock` (empty; the voice panel mounts here in its own pass) |
| `TBD_LobbyFactionRow.layout` | `30` | one faction / spectators row (`TBD_LobbyFactionRowComponent`) | `Border`, `Background`, `Name`, `RoleChipDock` (`DEFENDING`), `CountChipDock` (`0 / 92`) |
| `TBD_LobbyRoster.layout` | `31` | body of the ROLES panel | `ListFrame` (clips), `Scroll` (−24 overhang), `Content` (pad 34), `ScrollBarDock`, `EmptyState` |
| `TBD_LobbySquadCard.layout` | `32` | one collapsible squad (`TBD_LobbySquadCardComponent`) | `Border`, `Background`, `HeaderButton` (`HeaderOverlay` clips; `HeaderBG`, `CallsignChipDock`, `VehicleChipDock`, `CountChipDock`, `Chevron`), `HeaderRule`, `SlotsContent` |
| `TBD_LobbySlotRow.layout` | `33` | one seat (`TBD_LobbySlotRowComponent`) | `Background` (square image — rows sit inside the card), `RoleText`, `ChipsDock` (weapons + `MED`/`ENG` tags), `HolderText` (amber mono), `StatusChipDock` (`Unslotted` / `DEAD`), `RowRule` |
| `TBD_KitInspector.layout` | `34` | right column: title band + scrolling section cards | `PanelBorder`, `PanelBG`, `Header` (clips; `HeaderBG`, `HeaderIcon`, `Title`, `SlotRow`: `SlotTitle`, `SlotChipsDock`), `HeaderRule`, `BodyFrame` (clips), `Scroll`, `CardsContent`, `ScrollBarDock`, `EmptyState` |
| `TBD_KitPreview.layout` | `35` | the preview card (empty frame until the visual-preview pass) | `CardBorder`, `CardBG`, `Box` (`Border`, `Background`, `GridClip`/`GridImage`, `Label`) |
| `TBD_KitCell.layout` | `36` | one cell of a kit grid | `Border`, `Background`, `Label` (mono 10 upper), `Value` (mono 12), `Count` (amber `x4`) |
| `TBD_KitWeaponCard.layout` | `37` | one WEAPON SLOT card | `Border`, `Background`, `SlotChipDock`, `NameBorder`/`NameBG`/`Name`, `AttachmentsTitle`, `AttachmentsContent`, `AmmoRule`, `AmmoTitle`, `AmmoSummary`, `AmmoContent` (both contents take `TBD_KeyValueRow`s) |

Grids inside the kit cards are `Common/TBD_Columns3` / `TBD_Columns4` rows of `TBD_KitCell`
(GEAR / GADGETS / TOOLS / MISC four wide; GRENADES / MEDICAL three wide; WEAPONS three
`TBD_KitWeaponCard`s). Section cards are `Common/TBD_Panel`.

## Curves and grounds

| Surface | Radius | Ground it composites over |
|---|---|---|
| FACTIONS / ROLES columns (`PanelFill`), kit inspector panel | `RADIUS_PANEL` 12 | backdrop |
| faction rows, squad cards, weapon cards, preview card | `RADIUS_ROW` 8 | the owning panel's `GetGround()` / the card fill |
| squad header band | 7, bottom arcs clipped by `HeaderOverlay` (+8 px) | the card fill |
| kit cells, weapon name box, preview inner box, chips | `RADIUS_TAG` 6 | the card / weapon fill |
| slot rows | square (inside the rounded card) | `TBD_LobbySquadCardComponent.GetBodyGround()` |

Tokens live in `TBD_UITheme` (`FactionRowFill/Border/Ink(tint)`, `SLOT_*`, `SQUAD_*`, `KIT_*`,
`HOLDER_INK`, `BTN_SUCCESS_*`, `BTN_WARNING_*`).

## Behaviour

Faction click → roster rebuilds for that side, kit inspector clears. The selected faction wears a
solid tinted border and a lit fill; squad callsign chips wear the side's colour (blue / red).
Slot click → selected + kit shown; a second click on the selected OPEN seat claims it
(`[TBD][lobby] CLAIM …`); a second click on your own seat releases it. Holders are amber, your
own name too. Squad header click folds / unfolds the card (folds survive rebuilds; your own squad
never folds). The kit inspector resets its scroll on every rebuild (a shorter stack under an old
offset showed nothing until scrolled — measured). `Lock Lobby` ⇄ `Unlock Lobby` (amber),
`Ready & Continue` ⇄ `Ready (Waiting for Admin)` (emerald) — labels + tints only until the wire step.

## Line budget

Every file here is under 400 lines; the shell is 127. The old `TBD_LobbyInspector` (1312) and the
eight other monolith docks were deleted in this rebuild.
