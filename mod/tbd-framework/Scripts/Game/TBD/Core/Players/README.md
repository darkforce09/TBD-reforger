# Player lookups

Answers about a connected player that several systems derive the same way.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Core/Players/
└── TBD_PlayerFaction.c  the faction key of a player's assigned slot, or empty
```

## How it works

`TBD_PlayerFaction.Of(spawn, playerId)` asks `TBD_SpawnManager.GetAssignedSlot` and returns
the slot's `faction`. No spawn manager or no assigned slot returns the empty string, which callers
read as "on no side" rather than as an error.

## Authority

- Server: `TBD_PlayerFaction.Of` (`@authority server`); slot assignments live on the server.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` and
  `TBD_MissionSlotStruct` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`.
- Used by: `TBD_ObjectivesComponent` and `TBD_ObjectiveHudPublisher` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`, and `TBD_PlayAreaComponent`,
  `TBD_TriggerEffects` and `TBD_TriggerPlayerSnapshot` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`.
- Rules: lines added stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
