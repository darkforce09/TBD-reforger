# UI/layouts/Session/Shared — pre-game chrome

Layouts that two or more session screens wear unchanged. Handlers live in
`Scripts/Game/TBD/UI/Common/`; the base class that mounts them is
`Scripts/Game/TBD/UI/Core/TBD_DockScreen.c`.

| Layout | Handler | GUID block | Widget contract | API |
|---|---|---|---|---|
| `TBD_SessionTopBar.layout` | `TBD_SessionTopBar` | `1B` | `BarBorder`, `BarBG`, `Title`, `TabStripDock`, `IdentityBox` (`IdentityBorder`, `IdentityBG`, `IdentityName`, `IdentityRoleDock`), `CountBox` (`CountBorder`, `CountBG`, `CountIcon`, `CountText`) | `SetTitle(text, mono)`, `SetActiveTab(TBD_ESessionTab)`, `SetIdentity(TBD_SessionIdentity)`, `SetPlayerCount(n, cap)`, `GetOnTabSelected()(bar, tab)`, `GetStrip()` |
| `TBD_SessionBottomBar.layout` | `TBD_SessionBottomBar` | `1C` | `BarBorder`, `BarBG`, `LeftActions`, `RightActions` | `AddAction(id, label, primary, left)`, `SetActionLabel/Enabled/Visible/Primary`, `RemoveAll`, `FocusPrimary`, `GetOnAction()(bar, id)` |
| `TBD_PlayersPanel.layout` | `TBD_PlayersPanel` (`Session/Players/UI/`, a briefing mode in `WideDock`) | `43` | `Window` (fills the dock: `WindowBorder`, `WindowBG`, `Header`: `Title`, `TotalText`, `HeaderRule`), `BluforDock`, `OpforDock` (top 63 %), `SpectatorDock`, `UnslottedDock` (anchor-based, resize with the dock) | `Build(dock)` / `Destroy()` |
| `TBD_PlayerLane.layout` | `TBD_PlayerLane` (same file) | `44` | `Border`, `Background`, `HeaderClip`/`HeaderBG`, `NameText`, `RoleChipDock`, `CountLabel`, `CountChipDock`, `ColIndex`/`ColPlayer`/`ColPing`, `ColumnRule`, `ListDock` (a `TBD_ScrollList`) | `Build(dock, name, role, tint, rows, countLabel, countText)` |
| `TBD_PlayerRow.layout` | none (rows written by the lane) | `45` | `Index`, `Icon`, `Name`, `TagChipDock`, `Ping`, `RowRule` | ping ink SUCCESS < 40 ms, WARNING < 80, DANGER above |

## How a screen wears them

A screen's shell declares `TopDock` (56 px) and `BottomDock` (64 px). `TBD_DockScreen.OnScreenOpen`
mounts the two layouts there and wires them:

- title from `GetScreenTitle()` / `IsTitleMono()` — `SCENARIO BROWSER` shouted, or a scenario id
  such as `wog_187_chollima_on_the_wing_10` verbatim;
- active tab from `GetSessionTab()`; a click on another tab calls `OnTabSelected`, whose default
  is `TBD_MenuStack.Replace(PresetForTab(tab))` — Scenario Browser ↔ Lobby ↔ Briefing;
- identity + player count from `GetSessionIdentity()` (null hides the right cluster);
- bottom actions are the screen's: `GetBottomBar().AddAction("select_scenario", "Select Scenario", true)`
  and the click arrives in `OnBottomAction(bar, id)`.

The bar carries no buttons of its own, so the selector's single `Select Scenario`, the lobby's
`Lock Lobby` + `Ready & Continue` and the briefing's set are one layout with different rows. A
second `primary = true` demotes the earlier one and logs it (one loud button per screen).

## Status

- **Shipped:** both bars, worn by all three pre-game screens (selector 2026-09-12, lobby
  2026-09-13, briefing 2026-09-14); the players modal (2026-09-14, opened from the briefing's
  primary nav; any screen may open it).
- **Pending mockup for this folder:** `voice_panel` (`TBD_VoicePanel.layout`, mounts into the
  lobby's `VoiceDock`).
