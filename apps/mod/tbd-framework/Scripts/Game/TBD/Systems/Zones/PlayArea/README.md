# Play area enforcement

Keeps players inside the [mission](/documentation/glossary/g_to_m.md#mission)'s play area while the
round is live: a player outside a boundary zone, or inside another side's base-protection zone, is
warned in chat, given a grace countdown and then handed the zone's penalty.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/PlayArea/
├── TBD_PlayAreaComponent.c    game mode component: the 1 Hz enforcement tick over connected players
├── TBD_PlayAreaPenalties.c    the warning message and the none, warn and kill penalties
├── TBD_PlayAreaVehicleAxis.c  zoneRules.vehicleClasses: which occupant classes a zone confines
└── TBD_PlayAreaViolation.c    one player's open violation: countdown, warning cadence, penalty latch
```

## How it works

`TBD_PlayAreaComponent` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. Off a client, `OnPostInit` arms one
repeating 1 s tick; `OnDelete` cancels it and clears the zone registry. One tick that re-reads the
live player list is used rather than per-player timers, since `ScriptCallQueue.Remove` cancels by
function and a per-player callback could outlive its player onto a recycled id.

Each tick builds the registry when needed (logging once that no restriction is in force, or the
`armed` summary), drops every countdown outside `LIVE`, prunes rows of departed players and checks
each player:

- no body, a spent life or a dead body clears the player's row; with no `TBD_SpawnManager` the check
  stands down (one error line), since a spectator's streaming host would otherwise be policed;
- the violated zone is the governing boundary when the player is outside it, else another side's
  base-protection zone the player stands in;
- a new violation, a different zone or a different body restarts the countdown with a warning;
  otherwise the tick counts one second, warns every `warnEverySeconds`, and at the end of
  `graceSeconds` applies the penalty once;
- a player back inside who was warned is told so.

`TBD_PlayAreaPenalties` writes `TBD: you are outside the play area (<zone>) -- return within <n>s.`
(or "inside a protected area"), adding the one-life notice under `kill`, and says nothing under
`none`. At expiry `none` logs, `warn` logs and tells the player again, and `kill` ends the character
through the engine's `SCR_CharacterDamageManagerComponent.Kill`, self-instigated, so the death
reaches `TBD_SpawnManager` like any other; only an admin `#tbd respawn` brings the player back.

`TBD_PlayAreaVehicleAxis` binds each zone's `vehicleClasses` and classifies the occupant: on foot is
`infantry`, a helicopter or plane `aircraft`, a buoyant vehicle `sea`, anything else `ground`. An
empty list confines every class; a zone that leaves a class out does not confine its occupants.

## Authority

- Server: everything. `OnPostInit` arms the tick only off a client (`TBD_Authority.IsClient`); the
  component, penalties and axis carry `@authority server`.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ZoneRegistry` in `../Registry/` and `TBD_Zone` in `../`; `TBD_SpawnManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_Authority`,
  `TBD_AnnounceOnce`, `TBD_PlayerChat`, `TBD_PlayerFaction`, `TBD_CharacterUtil` and `TBD_Log` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's `SCR_CharacterDamageManagerComponent`
  and `SCR_CompartmentAccessComponent`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_PlayAreaComponent`; `TBD_ZoneCompiler` and `TBD_ZoneRegistry` in `../Registry/`
  (`TBD_PlayAreaVehicleAxis`).
- Rules: the default penalty stays `warn`; the penalty fires once per violation; enforcement stands
  down rather than police a body it cannot identify; `TBD_PlayAreaComponent` keeps its class name
  (the prefab references it); `cargo xtask mod compile` checks that the scripts compile, while
  whether a player is warned is checked in a round.

## Related documentation

- [Play area warning specification](/documentation/mod/tbd-framework/UI/play_area_warning/play_area_warning_specification.md)
  — the design of the out-of-bounds warning the player sees
