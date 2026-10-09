# Mission zones, play area and triggers

Turns a [mission](/documentation/glossary/g_to_m.md#mission)'s zones into prepared shapes the server
can test positions against, confines players to the authored play area with a warning, a grace
countdown and the authored penalty, and runs the mission's editor triggers and the effects they
fire. The objective system reads the same zones.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/
├── PlayArea/           play-area enforcement: the game mode component, warnings, penalties, vehicle classes
├── Registry/           the prepared zones of the loaded mission and the in-bounds questions
├── Triggers/           editorTriggers[]: preparation, conditions, the state machine and the effects
├── Volumes/            zoneRules height bounds, capture counts and starting owner for objectives
├── TBD_Zone.c          TBD_Zone: one prepared zone with its shape, rules and bounding box
└── TBD_ZoneGeometry.c  pure XZ containment maths for circles and polygons
```

## How it works

### Zones

`TBD_ZoneRegistry.Build()` in [Registry/](Registry/README.md) turns `TBD_MissionLoader.GetZones()`
into `TBD_Zone`s once per world through `TBD_ZoneCompiler`: the shape flattened to a circle or a
flat `[x0, z0, x1, z1, …]` polygon, the rules resolved and the bounding box computed, so a per-tick
test allocates nothing. `TBD_Zone.Contains` answers in 2D (the world XZ plane; height is ignored)
through `TBD_ZoneGeometry`, which counts a point within `EDGE_MARGIN_M` (1 m) of the edge as inside.
`TBD_ZoneGeometry` is hand-rolled rather than `Math2D.IsPointInPolygon` because the engine call's
behaviour on an edge or vertex is undocumented. `Clear()` drops the registry, since statics outlive
a world.

What in bounds means:

- a `boundary` zone with no `faction` applies to everyone; with a `faction`, to that side only;
- a player is in bounds when inside at least one boundary zone that applies to them, and a player
  to whom none applies, or any player in a mission with no boundary zone, is not restricted;
- a `base_protection` zone with a `faction` is violated by a player of another side inside it; one
  with no `faction` protects nobody and is reported and skipped;
- `TBD_PlayAreaVehicleAxis` narrows a zone to the occupant classes its `vehicleClasses` names
  (`infantry`, `ground`, `sea`, `aircraft`); an empty list confines every class, so leaving out
  `aircraft` is how an aircraft exemption is authored.

### Play area enforcement

`TBD_PlayAreaComponent` in [PlayArea/](PlayArea/README.md), a `SCR_BaseGameModeComponent` on
`mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, runs one 1 Hz server tick over the
connected players while the stage is `LIVE`. A player who leaves the area gets a private chat
warning repeated every `warnEverySeconds`, and a countdown of `graceSeconds`. When it runs out, the
zone's `penalty` applies: `warn` (the default) keeps warning, `none` only logs, and `kill` ends the
character through the engine's own `SCR_CharacterDamageManagerComponent.Kill`, which under one life
removes the player from the event. Returning inside clears the countdown, and a stage change clears
every countdown.

### Zone volumes

`TBD_ZoneVolumeBounds` in [Volumes/](Volumes/README.md) reads the objective half of `zoneRules`:
`minHeight` and `maxHeight` (height above the ground under the entity; an absent bound is open, and
min above max is an empty volume), `attackerCount`, `defenderCount`, `advantagePercent` and
`startingOwner`. The objective system calls `TBD_ZoneVolumeBounds.Read` and the `TBD_ZoneVolume`
queries (`ContainsAgl`, `ResolveActingFaction`, `EnemyContestsHold`, `HolderPresent`,
`ApplyStartingOwner`) to decide who captures, contests and holds.

### Editor triggers

`TBD_TriggerRuntime` in [Triggers/](Triggers/README.md) is ticked each second by
[`TBD_RuntimeHeartbeat`](../../Gamemode/Orchestrator/Heartbeat/README.md); on the server in a
framework world the tick reads `editorTriggers[]` with its own typed pass over the mission JSON and
prepares each trigger against the registry's zones:

```text
INERT (can never fire; reported at load)
ARMED ──condition holds──> PENDING ──timeoutSeconds──> FIRED ──repeat──> ARMED
                              └──condition stops──> ARMED
```

| Conditions | Effects |
|---|---|
| `present`, `not_present` (a live player of the owner side in the area) | `spawn` (up to 32 copies at one point), `delete` (with children) |
| `detected_by`, `seized_by` (sides in and near the area) | `end_mission` (the winner is logged), `set_objective` (completes it, with an owner) |
| `timer` (from arming), `objective_complete` | `hint` (private chat), `play_sound` (owner RPC), `set_variant` |

Other folders read trigger states through `TBD_TriggerRuntime.GetAll()`, `FindById()` and
`HasFired()`: audio emitters and spawn modules arm on a `FIRED` trigger, and the task state machine
follows them.

## Authority

- Server: everything but the sound playback. `TBD_PlayAreaComponent.OnPostInit` arms its tick only
  off a client (`TBD_Authority.IsClient`), and `TBD_RuntimeHeartbeat` ticks the trigger runtime only
  on the server in a framework world; the enforcement, registry, trigger and volume classes carry
  `@authority server`, and the registry and volume code are called only from them and from the
  objective system on the server.
- Client: nothing but the owner RPC below.
- Owner: `TBD_RpcDo_TriggerSound` plays a trigger's sound on the addressed client
  (`@authority owner`); on a host that is also a player, `TBD_PushTriggerSound` plays it locally.
- RPCs, on the modded `SCR_PlayerController`:
  - `TBD_RpcDo_TriggerSound`: Reliable, Owner (`@rpc Reliable Owner`); the sound event name.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` and `TBD_MissionJsonPass` (zones and the raw JSON),
  `TBD_MissionVariants` and `TBD_SpawnManager` (a player's slot and side) under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_FrameworkManager` (the stage and ending
  the round); the objective classes in `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`
  (for `objective_complete` and `set_objective`); `TBD_Log`, `TBD_AnnounceOnce`, `TBD_PlayerChat`,
  `TBD_PlayerFaction`, `TBD_CharacterUtil`, `TBD_DeclaredFactions` and `TBD_EntityQuery` under
  `mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's `SCR_ChatComponent`,
  `SCR_CharacterDamageManagerComponent`, `SCR_UISoundEntity` and world queries; the `zone`,
  `zoneRules` and `editorTrigger` definitions in `contracts/definitions/mission.schema.json`.
- Used by: `TBD_ObjectiveRegistry`, `TBD_ObjectivesComponent` and `TBD_TaskStateMachine` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_WinConditionEvaluator` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; `TBD_RuntimeHeartbeat` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`; `TBD_AudioEmitter` and
  `TBD_DynamicSpawner` beside this folder (`GetAll()` of zones and triggers); `TBD_EntityQuery`
  (`TBD_ZoneVolume.ContainsOrigin`); `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which
  attaches `TBD_PlayAreaComponent`.
- Rules: zones are built and tested on the server only; "no boundary applies" and "outside the
  boundary" stay separate verdicts, so a mission without an AO confines nobody; the default penalty
  stays `warn`; `TBD_PlayAreaComponent.OnDelete` clears the registry for the next world; lines added
  stay ASCII, and `cargo xtask mod compile` checks that the scripts compile, while whether a trigger
  fires or a player is warned is checked in a round.

## Related documentation

- [Play area warning specification](/documentation/mod/tbd-framework/UI/play_area_warning/play_area_warning_specification.md)
  — the design of the out-of-bounds warning the player sees
