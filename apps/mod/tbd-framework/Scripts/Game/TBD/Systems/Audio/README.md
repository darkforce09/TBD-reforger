# Mission audio emitters and music cues

Plays the sound a [mission](/documentation_v2/glossary.md#mission) authors: emitters that sound
while a player stands within their radius, armed when the round goes live or when their trigger
fires, and music cues played on the round's start and end and on each task's success or failure.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Audio/
└── TBD_AudioEmitter.c  the audio reader and tick, the client sound source entity, and the delivery RPCs
```

## How it works

A `modded class SCR_BaseGameMode` arms a one-second self-re-arming tick on the server in a
framework world. The tick reads the document's `audio` block once per mission id with its own
`JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()`: `emitters[]` (`id`, `x`, `z`,
optional `y`, `sound`, `radiusM`, `loop`, `triggerId`) and `musicCues[]` (`id`, `event`, `track`).
`event` is an Enforce keyword, so the pass rewrites that key to `cueEvent` in a copy of the JSON
before it binds.

- Cues: `mission_start` fires once when the stage reaches `LIVE`, `mission_end` once at `END`, and
  `task_succeeded` or `task_failed` when a task of `TBD_TaskStateMachine` moves into that state.
  Every cue with that event is pushed to every connected player.
- Emitters: while `LIVE`, an emitter with no `triggerId` arms at once, and one with a `triggerId`
  arms when that trigger of `TBD_TriggerRuntime` is `FIRED`; a `triggerId` that names no prepared
  trigger logs one WARNING and stays silent. Each emitter arms once and is pushed to the players
  connected at that moment.

On the client, `TBD_AudioEmitter.SpawnLocalSource` spawns one `TBD_AudioSourceEntity` per emitter
id at the emitter's position (the terrain surface when `y` is absent). Each frame the entity
measures the distance to the local player's controlled entity; inside `radiusM` it plays the sound
through `SCR_UISoundEntity.SoundEvent`, once for a one-shot and every 4 s for a loop. The sound
itself is 2D: the radius is what makes it positional, and the volume does not change with distance.

## Authority

- Server: reading the `audio` block, the tick, arming, and choosing what to send. The tick is armed
  only when `RplSession.Mode()` is not `RplMode.Client`; `OnGameStart`, `TBD_PushAudioEmitter`
  and `TBD_PushAudioCue` carry `@authority server`.
- Client: nothing beyond what its owner RPCs deliver.
- Owner: the addressed player's client spawns the local sound source and plays cues. On a host
  that is also a player, the push plays locally without an RPC.
- RPCs, on the modded `SCR_PlayerController`:
  - `TBD_RpcDo_AudioEmitter`: Reliable, Owner (`@rpc Reliable Owner`); starts one emitter.
  - `TBD_RpcDo_AudioCue`: Reliable, Owner (`@rpc Reliable Owner`); plays one music track.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (the raw JSON and the mission id); `TBD_FrameworkManager` (the
  stage); `TBD_TaskStateMachine` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`
  (task states); `TBD_TriggerRuntime` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`
  (trigger states); `TBD_Log`; the engine's `SCR_UISoundEntity`, `SCR_PlayerController` and
  `PlayerManager`.
- Used by: nothing outside the folder calls it; the game mode runs it through the modded
  `SCR_BaseGameMode`, and each client through the modded `SCR_PlayerController`.
- Rules: the server alone reads the document, and clients act only on what an owner RPC delivers;
  presence is `Count()` on `emitters` and `musicCues`, never a null test on `audio`; each emitter
  arms and each start or end cue fires once per mission; lines added stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while whether a sound plays in a round
  is checked by hand.
