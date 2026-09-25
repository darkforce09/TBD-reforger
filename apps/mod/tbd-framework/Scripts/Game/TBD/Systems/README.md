# Framework systems

The in-world machinery of the TBD framework [mod](/documentation_v2/glossary.md#mod): loading the
[mission](/documentation_v2/glossary.md#mission) deployed to the server, standing slot bodies in the
world and deploying players onto them, dressing loadouts, and running the mission's zones,
triggers, AI, audio, map markers and radio nets. The stage machine, objectives and win conditions
in `Gamemode/` and the player-facing screens in `Session/` build on it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/
├── AI/         waypointed AI groups armed at the live round, and group combat, formation and speed
├── Audio/      mission sound emitters and music cues, played on the clients
├── Loadouts/   the server equip pass for slot bodies, the lobby kit preview, a dev equip harness
├── Markers/    each side's briefing markers served to its players and drawn on the in-game map
├── Mission/    the deployed mission's load, verification, validation, event roster and state readers
├── Radio/      each side's radio nets served, tuned into carried radios and shown to players
├── Spawning/   slot bodies, deploys, one life, platform deployment authorization, AI spawn modules
└── Zones/      prepared zones, play area enforcement, zone volumes and editor triggers
```

## How it works

`Mission/` runs first: once per world, on the server, it loads the deployed artifact, verifies its
SHA-256, validates the document and reads the [event](/documentation_v2/glossary.md#event) roster.
Every other folder reads the loaded document through `TBD_MissionLoader`, either its typed structs
or its raw JSON in a second `JsonLoadContext` pass of its own.
`Spawning/` then materializes one body per slot, dressed by `Loadouts/`, placed with
`Mission/Ingestion/` scatter and handed to players under one life. `Zones/`, `AI/`, `Audio/` and the
spawn modules tick once a second on the server while the round is live, and `Markers/` and `Radio/`
answer each client's pull with its own side's data only.

Three patterns hold across the folders:

- Wiring: a folder whose code needs a place on the game mode is a `SCR_BaseGameModeComponent` on
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` (`TBD_SpawnManager`,
  `TBD_LoadoutEquipComponent`, `TBD_PlayAreaComponent`, `TBD_MarkerComponent`,
  `TBD_RadioComponent`); the tickers (AI, audio, weather, triggers, spawn modules) add a
  `modded class SCR_BaseGameMode` whose `OnGameStart` arms a self-re-arming tick, fenced by
  `TBD_FrameworkManager.IsFrameworkWorld()` so a vanilla scenario with the mod loaded runs none of
  it.
- Presence: `JsonLoadContext` allocates a nested `ref` field even when its key is absent, so every
  folder tests presence with a sentinel, an empty string or a `Count()`, never a null test.
- Statics outlive a world inside one process, so each folder clears its static state when the
  game mode starts or its component is deleted.

## Authority

- Server: the mission document, slot bodies, deploys, loadouts, zones, triggers, AI, spawn
  modules, weather, and the decisions of which markers and nets a player gets. Clients never hold
  or parse the mission document.
- Client: the marker and radio pull loops, the kit preview, the Ready & Continue answer, and the
  sound sources.
- Owner: every server-to-client message is an owner RPC on `SCR_PlayerController`, answering the
  requesting or addressed player only.
- RPCs, all Reliable on the modded `SCR_PlayerController`:
  - `Audio/`: `TBD_RpcDo_AudioEmitter` and `TBD_RpcDo_AudioCue`, Owner;
  - `Markers/`: `TBD_RpcAsk_Markers`, Server, and `TBD_RpcDo_Markers`, Owner;
  - `Radio/`: `TBD_RpcAsk_RadioNets`, Server, and `TBD_RpcDo_RadioNets`, Owner;
  - `Spawning/`: `TBD_RpcAsk_ReadyDeploy`, Server, and `TBD_RpcDo_ReadyDeployResult`, Owner;
  - `Zones/`: `TBD_RpcDo_TriggerSound`, Owner.
- Replicated properties: none.

## Boundaries

- Depends on: `apps/mod/tbd-framework/Scripts/Game/TBD/Core/` (logging, hashing, the prefab
  registry); `apps/mod/tbd-framework/Scripts/Game/TBD/API/` (the platform bridge, the runtime
  session, player identity); `TBD_FrameworkManager` and the objective classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; over HTTP with the `mod_runtime`
  [machine credential](/documentation_v2/glossary.md#machine-credential), the API's
  [game runtime](/documentation_v2/glossary.md#game-runtime) deployment, artifact, roster and
  deployment-authorization routes; the wire shapes in `contracts_v2/definitions/`.
- Used by: the stage machine, objectives and win conditions in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`; the admin, briefing, lobby, players,
  post-game and spectator code in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the HUD in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the platform reports in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the components.
- Rules: the server alone reads the mission document, and a client learns only its own side's
  markers and nets; every modded vanilla class does its work only in a framework world; an RPC
  carries at most eight parameters, the `Rpc()` limit; wire structs keep their JSON keys'
  spelling with a `@contract` tag (`cargo xtask schema citations`); sources stay ASCII, and
  `cargo xtask mod compile` checks that they compile.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — what the framework is for, its
  non-negotiables and the Enfusion facts it relies on
- [Mod documentation](/documentation_v2/mod/README.md) — the index of the mod's deeper documents
