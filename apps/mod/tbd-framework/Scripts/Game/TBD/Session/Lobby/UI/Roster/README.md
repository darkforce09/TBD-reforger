# Lobby ROLES column

The middle column of the Lobby screen: one card per squad of the chosen faction, each with its
seat rows, where a click selects a seat, a second click claims it, and a second click on your own
seat gives it back. The briefing's ORBAT page reuses it read-only with a Locate button per squad.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/Roster/
├── TBD_LobbyRosterPanel.c          the column: rebuilds the cards, selection, claim and release
├── TBD_LobbySlotRowComponent.c     one seat row: role, chips, holder, DEAD or Unslotted status
└── TBD_LobbySquadCardComponent.c   one squad card: callsign, vehicle and fill chips, fold toggle
```

## How it works

`TBD_LobbyRosterPanel.Build` mounts `TBD_LobbyRoster.layout` into a panel and subscribes to
`TBD_LobbyCatalog.GetOnChanged`. `SetFaction` and every catalog change rebuild all cards: one
`TBD_LobbySquadCardComponent` (`TBD_LobbySquadCard.layout`) per squad, its callsign chip in the
side tint, and one `TBD_LobbySlotRowComponent` (`TBD_LobbySlotRow.layout`) per seat. A folded card
stays folded across rebuilds unless it holds your own seat. A seat click selects it and raises
`GetOnSelected(panel, slotKey)`; a second click on the selected OPEN seat calls
`TBD_LobbyCatalog.Claim`, and a second click on your own seat calls `Release`, each logged as
`[TBD][lobby] CLAIM` or `RELEASE`. `SetReadOnly(true)` limits clicks to selection and
`SetShowLocate(true)` adds a Locate button to each squad header, raising `GetOnLocate(panel,
callsign)`.

## Authority

- Server: nothing.
- Client: everything; the column runs on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_LobbyCatalog` and its seat and squad models in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Catalog/`; `TBD_UILayouts`, `TBD_UITheme`,
  `TBD_UIInteractive`, `TBD_UIScrollBar` and `TBD_UIButton` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_PanelComponent` and `TBD_ChipComponent`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Lobby/`.
- Used by: `TBD_LobbyScreen` in the parent folder; `TBD_BriefingOrbatPage` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Pages/`.
- Rules: `TBD_LobbySlotRowComponent` and `TBD_LobbySquadCardComponent` keep their class names
  (the layouts reference them); a DEAD seat is never interactive; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.
