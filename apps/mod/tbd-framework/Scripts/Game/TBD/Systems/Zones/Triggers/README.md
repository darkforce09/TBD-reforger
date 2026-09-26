# Editor triggers

Reads the triggers an author draws in the Mission Creator (`editorTriggers[]`), prepares each one
against the mission's zones once per world, and while the round is live checks each trigger's
condition once a second and fires its effects when the condition has held long enough.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/Triggers/
├── TBD_EditorTriggerStruct.c     the editorTriggers[] wire records: trigger, activation, effects, params
├── TBD_Trigger.c                 TBD_Trigger and TBD_TriggerEffect: one prepared trigger and its state
├── TBD_TriggerCompiler.c         reads editorTriggers[] and prepares each trigger against the zone registry
├── TBD_TriggerConditions.c       whether a trigger's condition holds this tick
├── TBD_TriggerEffectValidator.c  decides at load whether each effect can ever run
├── TBD_TriggerEffects.c          effect dispatch, hint and play_sound, and audience selection
├── TBD_TriggerFlowEffects.c      set_objective, set_variant and end_mission
├── TBD_TriggerPlayerSnapshot.c   one tick's live players: position and side
├── TBD_TriggerRuntime.c          the registry, the once-a-second state machine and variant selection
├── TBD_TriggerSoundRelay.c       carries a play_sound cue to each client; the owner RPC
├── TBD_TriggerVocabulary.c       the condition and effect vocabularies and their enums
└── TBD_TriggerWorldEffects.c     spawn and delete
```

## How it works

### Reading and preparing

`TBD_TriggerRuntime.Tick` is called every `TICK_MS` (1 s) by
[`TBD_RuntimeHeartbeat`](../../../Gamemode/Orchestrator/Heartbeat/README.md), which runs only on the
server in a framework world; `TBD_RuntimeHeartbeat` also calls `Clear` at each world start. Clients
hold no mission document, so nothing here would find a trigger on a client.

The first tick after `Clear` calls `Build`, which waits until the zone registry is built, then asks
`TBD_TriggerCompiler.ReadWire` for `editorTriggers[]`. `TBD_MissionDocumentStruct` does not declare
that key, and `JsonLoadContext` maps JSON keys onto named class fields only, so the compiler runs a
second typed pass (`TBD_MissionJsonPass.LoadRoot`) with a root, `TBD_TriggerDocStruct`, that
declares `editorTriggers` and nothing else. The document is parsed once per world.

`TBD_TriggerCompiler.Prepare` turns each wire record into a `TBD_Trigger`:

- `activation.condition` maps through `TBD_TriggerVocabulary`; an absent or unknown condition, or a
  `seized_by` or `detected_by` with no `ownerSide`, makes the trigger INERT with its reason.
- `timeoutSeconds` is a dwell: the condition must hold without a gap for that long, and a gap resets
  it. A negative dwell is logged and read as 0.
- `zoneId` binds to the registry's own `TBD_Zone` (`TBD_ZoneRegistry.FindById`), so the trigger's
  area and the play area's are the same object. An absent `zoneId` is a document-wide condition
  (except `detected_by`, which needs an area); an id that names no zone, or a zone with no usable
  shape, makes the trigger INERT rather than world-wide.
- Each effect is flattened and checked by `TBD_TriggerEffectValidator`. `params` is an open object
  in the schema, so a misspelled param arrives as an absent one; every unusable effect names the
  param it wanted. `count` is clamped to 1..`MAX_SPAWN_COUNT` (32) and an unknown `state` reads as
  `complete`, both logged. A trigger with no usable effect is INERT.

`JsonLoadContext` allocates a nested `ref` field even when its key is absent, so presence is read
from field values: numbers carry the `TBD_TriggerParamsStruct.ABSENT` sentinel (`-1000000`, since 0
is a real coordinate) and containers are tested by count. `repeat` is a bool, so absent and authored
`false` are the same value, which is also the schema default.

Each INERT trigger is logged by id, each armed one gets a `trigger` line, and `Build` ends with one
`built` summary; the first tick after it logs `armed` once, or that no trigger can fire.

### The tick

```text
INERT (can never fire; reported at load)
ARMED ──condition holds──> PENDING ──timeoutSeconds──> FIRED ──repeat──> ARMED
                              └──condition stops──> ARMED
```

- The registry remembers the mission id it was built for. When the loaded mission changes, the tick
  logs it and calls `Clear` rather than fire another mission's triggers; `Clear` at world start is
  the other defence, since statics outlive a world when a mission restarts in-process.
- Only the `LIVE` stage evaluates; any other stage returns every PENDING trigger to ARMED, so a
  dwell never resumes half-way.
- `TBD_TriggerPlayerSnapshot.Capture` records each connected player with a live body and an unspent
  life: position and slot faction. A dead player's controlled entity is their spectator host, not a
  soldier, so it is left out. Without a `PlayerManager` or `TBD_SpawnManager` the capture fails and
  the tick stands down with one error line, because an empty snapshot would make every
  `not_present` trigger hold.
- `TBD_TriggerConditions.Holds` answers the condition. `present` and `not_present` count players of
  the owner side in the area (an empty owner side matches everybody); `not_present` also refuses to
  hold while an unidentified body (no slot yet, or mid-respawn) is in the area. `seized_by` needs
  the owner side in and no other named side. `detected_by` is proximity, not perception: a player of
  another side inside the area with an owner-side player within `DETECT_RADIUS_M` (250 m) of them,
  measured in XZ with no line of sight traced; the observer need not be inside. `timer` always
  holds, so its dwell counts from arming. `objective_complete` reads one objective (the `zoneId`) or
  every usable objective, and at least one must exist.
- A `repeat` trigger re-arms only once its condition stops holding, so it fires again rather than
  continuously; a repeating `timer` re-arms at once and is periodic. A spent one-shot is skipped
  without evaluating its condition.

### Variants

`editorTrigger.variantId` arrives on the wire verbatim. The runtime starts with no selection in
force, so every trigger runs whatever its `variantId`. The first `set_variant` effect calls
`SelectVariant`, which puts selection in force (logged once): from then on a trigger with a
`variantId` runs only while that variant is selected (`TBD_MissionVariants.IsActive`), and one
without a `variantId` always runs. A trigger whose variant is not selected drops its dwell.

### Effects

`Fire` latches the trigger FIRED, logs a banner and runs every usable effect in authored order, so a
`hint` written before an `end_mission` is delivered first. Every effect logs one line naming the
trigger.

- `hint`: a private chat line `TBD: <text>` through `TBD_PlayerChat.Tell`; the text is logged too.
- `play_sound`: the server has no audio device, so `TBD_TriggerSoundRelay` pushes the
  `SCR_SoundEvent` name to each addressed client's `SCR_PlayerController`, which raises it as a 2D
  UI sound. An unknown event name raises nothing, so the authored name is logged.
- The audience comes from the live player list, so dead players are still told: empty or `all` is
  everybody, `owner` the activation's side, `enemy` every other named side, and anything else a
  faction key. `owner` or `enemy` without an owner side addresses nobody.
- `spawn`: `count` copies of the registry alias at one point, `params.x`/`params.z` or the zone's
  centre, snapped to the ground, the same placement `TBD_MissionLoader` gives authored entities.
- `delete`: every entity of the alias whose origin is inside the zone's drawn shape, with its
  children (`TBD_EntityQuery.CollectPrefabInZone`, then `SCR_EntityHelper.DeleteEntityAndChildren`).
  A `delete` with no zone is refused at load.
- `set_objective`: sets the objective's `m_bComplete`, the flag the objective system itself sets, and
  `params.owner` as its capture owner, so a trigger-driven completion ends the round the same way.
- `end_mission`: logs `[TBD][Win] trigger:<id> - winner=<winner>` and moves the stage to END through
  `TBD_FrameworkManager.SetStage`, the only door to the stage; a refusal is logged.

## Authority

- Server: everything but the sound playback. `TBD_RuntimeHeartbeat` ticks the runtime only on the
  server in a framework world, and the trigger classes carry `@authority server`.
- Client: nothing but the owner RPC below.
- Owner: `TBD_RpcDo_TriggerSound` raises the sound on the addressed client (`@authority owner`); on
  a listen host, `TBD_PushTriggerSound` plays its own cue in place, since an owner RPC is not
  delivered to the machine that sends it.
- RPCs, on the modded `SCR_PlayerController` in `TBD_TriggerSoundRelay.c`:
  - `TBD_RpcDo_TriggerSound`: Reliable, Owner (`@rpc Reliable Owner`); the sound event name.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ZoneRegistry` and `TBD_Zone` beside this folder; `TBD_MissionJsonPass`,
  `TBD_MissionLoader`, `TBD_MissionVariants`, `TBD_Registry` and `TBD_SpawnManager` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_FrameworkManager` and
  `TBD_ObjectiveRegistry` under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; `TBD_Log`,
  `TBD_AnnounceOnce`, `TBD_PlayerChat`, `TBD_PlayerFaction`, `TBD_CharacterUtil` and
  `TBD_EntityQuery` under `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's
  `SCR_UISoundEntity` and `SCR_EntityHelper`; `#/$defs/editorTrigger` in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_RuntimeHeartbeat` (`Clear`, `Tick`, `TICK_MS`); `TBD_TaskStateMachine`,
  `TBD_AudioEmitter` and `TBD_DynamicSpawner` (`IsBuilt`, `GetAll`, `FindById`, `HasFired` and the
  `TBD_Trigger` fields).
- Rules: a trigger that cannot fire is INERT with a logged reason, never widened or guessed; a
  registry built for another mission never evaluates; evaluation stands down when presence cannot be
  read; the round ends only through `SetStage`; `cargo xtask mod compile` checks that the scripts
  compile, while whether a trigger fires is checked in a round.
