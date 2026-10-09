# Lobby screen catalog

The data and intents the Lobby screen reads: the factions with their seat counts, each faction's
squads and seats with holders, weapon chips and role tags, and the kit sheet behind every seat. It
is shaped like the lobby roster plus the presentation the roster does not carry.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Catalog/
├── TBD_KitEntry.c           one labelled kit line: label, value, count, tint
├── TBD_KitInfo.c            one seat's kit sheet and the kit alias and loadout its preview dresses
├── TBD_KitWeapon.c          one weapon card: name, attachments, ammunition
├── TBD_LobbyCatalog.c       the catalog: lookups, `Claim`, `Release`, the change event
├── TBD_LobbyFactionInfo.c   one FACTIONS row: name, role label, tint, seat count
├── TBD_LobbySlotInfo.c      one seat: role, chips, state, holder, kit key
└── TBD_LobbySquadInfo.c     one squad card: callsign, vehicle, seats
```

## How it works

`TBD_LobbyCatalog.Get()` returns the process-wide catalog, building it from `TBD_LobbyMock` on
first use; `Set()` replaces it and `Set(null)` restores the mock on the next `Get()`. No script
calls `Set()`, so every screen shows the mock. Screens hold the catalog, never a raw array, and
listen to `GetOnChanged()`. `Claim` takes an OPEN seat for the viewer, giving up the current one
first; `Release` gives it up; both change this copy only and raise the change event.

`TBD_LobbySquadInfo.AddSlot` keys each seat `<callsign>:<index>`. `TBD_KitInfo.BasePrefab` resolves
the kit alias through `TBD_Registry` (the addon ships its registry, so this works on clients); the
alias and loadout are what the server's slot body spawn applies, so the preview wears what the seat
spawns.

## Authority

- Server: nothing.
- Client: everything; the catalog is local UI data.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_LobbyMock` in `mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`;
  `TBD_SessionSelection` in `mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`;
  `TBD_Registry` in `mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_SlotLoadoutStruct` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`.
- Used by: the Lobby screen and its panels in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/`; `TBD_BriefingScreen` and
  `TBD_BriefingOrbatPage` in `mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`;
  `TBD_PlayersPanel` in `mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/`.
- Rules: every change raises `GetOnChanged`; seat state uses the wire vocabulary `OPEN`, `HELD`,
  `DEAD`; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.
