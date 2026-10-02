# Detailed match events

Records what happens in a live round, one detailed event at a time, for the platform's
[match telemetry](/documentation/glossary/g_to_m.md#match-telemetry): kills and deaths,
players going unconscious and coming round, vehicles destroyed, entered and left. It also keeps
the round's per-player combat tally that the match results and the DEBRIEF scoreboard report.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/MatchEvents/
├── SCR_VehicleDamageManagerComponent.c  modded vehicle damage manager: reports a vehicle entering the destroyed state
├── TBD_MatchCharacterWatch.c            one player character's life-state and seat subscriptions
├── TBD_MatchEventCapture.c              engine facts to events and tally credits: identities, relation, distance, weapon, cause
├── TBD_MatchEventRecorder.c             the LIVE round's event buffer, its mission clock and the 10 s batch flush
├── TBD_MatchEventWire.c                 the `MatchEvent` object and one JSON payload builder per event kind
├── TBD_MatchTelemetryComponent.c        game mode component: kill, death, spawn and deletion hooks
└── TBD_MatchTelemetryTally.c            per-player kills, team kills, longest kill and vehicles destroyed this round
```

## How it works

```text
TBD_ResultsReporter at LIVE   -> TBD_MatchRegistration.BeginRound -> TBD_MatchEventRecorder.BeginRound
                                  (source match id, mission clock start, tally cleared)
engine hooks (server only)    -> TBD_MatchEventCapture -> TBD_MatchTelemetryTally (credits)
                                                      -> TBD_MatchEventRecorder.Capture (kind, payload)
TBD_RuntimeHeartbeat, 10 s    -> TBD_MatchEventRecorder.Tick  -> ReserveEventSequences -> TBD_MatchEventBatch.Enqueue
TBD_ResultsReporter at END    -> TBD_MatchEventRecorder.EndRound (flush) -> results revision reads the tally
```

The hooks and the events they become:

| Engine hook | Condition | Event | Tally |
|---|---|---|---|
| `OnPlayerKilled` | another player killed the player | `combat.kill` (`victim_is_player` true) | kill, or team kill |
| `OnPlayerKilled` | no player killer, or the player's own hand | `combat.death` (`ai`, `environment`, `self`, `unknown`) | none |
| `OnControllableDestroyed` | a player killed an AI character | `combat.kill` (`victim_is_player` false) | kill, or team kill |
| `m_OnLifeStateChanged` | to `INCAPACITATED` | `medical.incapacitated` | none |
| `m_OnLifeStateChanged` | `INCAPACITATED` to `ALIVE` | `medical.revived` | none |
| `GetOnCompartmentEntered` / `GetOnCompartmentLeft` | not a seat move | `vehicle.entered` / `vehicle.exited` | none |
| `SCR_VehicleDamageManagerComponent.OnDamageStateChanged` | into `EDamageState.DESTROYED`, not a join-in-progress state | `vehicle.destroyed` | vehicle destroyed, for the instigating player |

- A team kill is the engine's `KILLED_BY_FRIENDLY_PLAYER` relation; it is counted apart from
  kills and never toward the longest kill. The distance is killer to victim in whole metres; the
  weapon is the killing character's current weapon prefab, omitted when the killer is seated in a
  vehicle or holds none.
- The `combat.death` cause is `self` when the killer is the victim, `ai` for the
  `KILLED_BY_ENEMY_AI` or `KILLED_BY_FRIENDLY_AI` relation, `environment` without a killer entity,
  and `unknown` otherwise.
- A player's death reaches only `OnPlayerKilled`: `OnControllableDestroyed` skips a character a
  player spawned into, controls, or the context names a player for.
- Identities come from `TBD_PlayerIdentity.GetArmaId`, the bytes the results lines carry. An
  event whose required identity is missing is skipped; an optional one (`victim_arma_id`,
  `instigator_arma_id`) is omitted. The tally counts by player id whether or not an identity
  exists.
- Each spawned player character gets a `TBD_MatchCharacterWatch`, found through the game mode's
  `GetOnPlayerSpawned` invoker, which the framework's deploys also raise; `OnControllableDeleted`
  detaches it, and the component's `OnDelete` detaches every watch.
- Recording runs only while a registered round is `LIVE`: the recorder holds the source match id
  of `TBD_MatchRegistration`, and `TBD_FrameworkManager.GetStage()` must read `LIVE`. The tally is
  credited under the same condition. `mission_time_ms` counts from the recorder's `BeginRound`;
  `occurred_at` is the capture time in RFC 3339 UTC.
- The flush reserves one run of sequences for everything buffered, in capture order, so
  `sequence` is the capture order and `event_id` is the sequence as a decimal string; it queues
  batches of at most 100 events. A full buffer of 100 flushes at once; `EndRound`, a new round and
  the game-start `Clear` flush before they forget the round.

## Authority

- Server: everything; the component's hooks, the modded damage manager and the recorder return
  on a client, and `TBD_MatchEventRecorder.IsRecording` is false there.
- Client: nothing; `TBD_MatchEventRecorder.Clear` runs at game start on every machine and has
  nothing to flush on a client.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the DEBRIEF board built from the tally is replicated by
  `TBD_FrameworkManager`.

## Boundaries

- Depends on: `TBD_MatchEventBatch`, `TBD_MatchRegistration` and `TBD_TelemetryQueue` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/`; `TBD_PlayerIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Identity/`; `TBD_BackendText` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_FrameworkManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_LoadoutInventoryUtil.PrefabOf`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/`; `TBD_Authority`, `TBD_Log` and
  `TBD_Rounding` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's
  `SCR_InstigatorContextData`, `SCR_CharacterControllerComponent`,
  `SCR_CompartmentAccessComponent`, `BaseCompartmentSlot`, `BaseWeaponManagerComponent` and
  `SCR_VehicleDamageManagerComponent`; the `MatchEvent` definitions in
  `contracts/definitions/match-telemetry.schema.json`.
- Used by: `TBD_ResultsReporter` and `TBD_ResultsPayload` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/`; `TBD_RuntimeHeartbeat` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`;
  `TBD_DebriefScoreboard` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/PostGame/`; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_MatchTelemetryComponent`.
- Rules: every payload builder carries `@contract match-telemetry.schema.json#/definitions/<Name>`
  and escapes every string through `TBD_BackendText.JsonEscape`; a `string.Format` takes at most
  nine arguments; lines added stay ASCII, and `cargo xtask mod compile` checks that the scripts
  compile, while the events themselves are checked in game.

## Related documentation

- [Match telemetry design](/documentation/apps/api/verification_evidence/telemetry.md) — the seven
  event kinds, their payloads and how the API stores them
- [Match telemetry transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/MatchTelemetry/README.md) — the
  durable queue that carries the batches
