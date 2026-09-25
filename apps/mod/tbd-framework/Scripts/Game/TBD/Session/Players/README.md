# Shared player data

Who is connected, shaped for the screens that show players: the briefing's PLAYERS panel and the
slotted-count badges on its navigation.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/
├── TBD_PlayersCatalog.c  `TBD_PlayerInfo` and `TBD_PlayersCatalog`: players by state and faction
└── UI/                   the PLAYERS panel of the briefing
```

## How it works

`TBD_PlayerInfo` holds a player's name, faction key (`BLUFOR`, `OPFOR` or empty), ping, an optional
tag such as `ADMIN`, and a `TBD_EPlayerState` of `SLOTTED`, `SPECTATOR` or `UNSLOTTED`.
`TBD_PlayersCatalog` holds the players and the seat capacity per faction, and answers `Total`,
`Capacity`, `GetSlotted(faction)`, `GetByState`, `CountSlotted` and the totals across factions.
Screens read `TBD_PlayersCatalog.Get()`, which builds the catalog from `TBD_PlayersMock` in
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` on first use; `Set()` replaces it, and no script
calls `Set()`, so every screen shows the mock players. The live data it would need, the player
manager and `TBD_SpawnManager`'s slot map, exists only on the server.

## Authority

- Server: nothing.
- Client: everything; the catalog and the panel live on the local machine.
- Owner: nothing.
- RPCs: none; no transport carries player data to the catalog.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_PlayersMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`; the shared
  UI library in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: `TBD_BriefingScreen` and `TBD_BriefingPrimaryNav` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`.
- Rules: screens read player data only through `TBD_PlayersCatalog.Get()`, so a live source
  replaces the mock with one `Set()` call; `cargo xtask mod compile` checks that the scripts
  compile.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the briefing's design target, the players panel included
