# Mission radio nets

Gives each player their side's radio nets from the [mission](/documentation_v2/glossary/g_to_m.md#mission)'s
radio plan: the server picks the nets for the player's side, tunes them into the radios the player
carries and verifies each tune by reading it back, and the client shows the list with the truth
about what was tuned.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/
├── TBD_RadioBridgeStub.c   partner voice bridge hooks, and the stage-change call that starts the radio sweep
├── TBD_RadioClient.c       the client's pull loop, the net list it holds, and the hint that shows it
├── TBD_RadioComponent.c    game mode component that starts the client and reports the radio backbone
├── TBD_RadioController.c   the request, reply and push RPCs on the player controller
├── TBD_RadioPlan.c         radioPlan.nets[] structs and the validated nets of each faction
├── TBD_RadioService.c      which nets a player gets, the stage sweep, and the wire
└── TBD_RadioTuner.c        tunes a player's radios to their nets and reads each frequency back
```

## How it works

`TBD_MissionLoader` binds `radioPlan` into `TBD_MissionRadioPlanStruct` in its primary parse.
`TBD_RadioPlan` validates it by content, since `JsonLoadContext` allocates the block even when it
is absent: the plan exists when `nets.Count()` is above 0, and a net counts when its `freqMHz` is
inside 30..512 and its `id` and `label` are not empty. `GetNetsForFaction` builds the list of the
side's nets plus every net with no `faction`, which all sides share.

```text
TBD_RadioComponent.OnPostInit (not a dedicated server) ──2.5 s──> TBD_RadioClient.Start
TBD_RadioClient: every 5 s until served, and on each map open (at most every 3 s)
  └─> SCR_PlayerController.TBD_RequestRadioNets ──TBD_RpcAsk_RadioNets──> server
        TBD_RadioService.BuildForPlayer(playerId): side = TBD_SpawnManager.GetAssignedSlot
          └─> TBD_RadioTuner.TunePlayer: SetFrequency (kHz) on each transceiver, then read back
        ──TBD_RpcDo_RadioNets──> owner client ──> TBD_RadioClient.Accept ──> hint (popup if hints off)
TBD_FrameworkManager stage change ──> TBD_RadioBridgeStub.OnStageChanged ──> TBD_RadioService.OnStageChanged
  at SAFE_START and LIVE: build, tune and TBD_PushRadioNets for every connected player
```

- Side discipline matches the markers: the request carries no argument, the side comes from the
  server's slot assignment, and only the player's nets leave the server. A player with no slot
  gets `served` false and keeps asking.
- The wire: parallel arrays (`ids`, `labels`, `freqKHz`, `longRange` as 0 or 1) plus the mission
  id, the tune result, the count of verified tunes and `served`: eight parameters, the `Rpc()`
  limit. Frequencies travel as integer kHz, the engine radio API's unit.
- Tuning: the tuner walks the body's `RADIO` and `RADIO_BACKPACK` gadgets and their transceivers,
  preferring a backpack radio for a `long` net, and counts a tune only when `GetFrequency` reads
  back what was set. `TBD_ERadioTuneResult` names the outcome (`NO_BODY`, `NO_RADIO`,
  `READBACK_MISMATCH`, `TUNED` and the rest). A world without a `RadioManagerEntity` still tunes
  through `FallbackChannelTable`, and `TBD_RadioComponent` logs once per world whether the entity
  exists.
- Display: `TBD_RadioClient` shows the net list as a vanilla hint that stays until dismissed, and
  the text says the frequencies must be dialled by hand when no tune was verified. Its
  `GetNetLine` and `GetNetCount` accessors have no caller outside the folder.
- `TBD_RadioBridgeStub` also declares `OnPlayerSpawned`, `OnPlayerSpawnedById`, `OnPlayerKilled`,
  `OnRadioRetune` and `OnPTT` for a partner voice bridge; nothing calls them, and `OnPlayerKilled`
  and `OnPTT` are empty.

## Authority

- Server: `TBD_RadioPlan`, `TBD_RadioService`, `TBD_RadioTuner` and the stub's `OnPlayerSpawned`,
  `OnPlayerSpawnedById`, `OnRadioRetune` and `OnStageChanged`, each tagged `@authority server`; `OnStageChanged` and `TBD_PushRadioNets` return on `RplMode.Client`. On a
  listen host, `TBD_RequestRadioNets` and `TBD_PushRadioNets` hand the wire to the local client
  without an RPC.
- Client: `TBD_RadioClient` (`Start` carries `@authority client`), started by
  `TBD_RadioComponent` on every machine that is not a dedicated server.
- Owner: `TBD_RpcDo_RadioNets` and `TBD_RadioClient.Accept` run on the addressed client only
  (`@authority owner`).
- RPCs, on the modded `SCR_PlayerController`:
  - `TBD_RpcAsk_RadioNets`: Reliable, Server (`@rpc Reliable Server`); takes no argument.
  - `TBD_RpcDo_RadioNets`: Reliable, Owner (`@rpc Reliable Owner`); the reply and the stage push.
- Replicated properties: none; the engine replicates the transceiver frequencies the server sets.

## Boundaries

- Depends on: `TBD_MissionLoader` (the `radioPlan` field) and `TBD_SpawnManager` (the caller's
  slot) under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_FrameworkManager` (stage
  changes); `TBD_Log`; the engine's `SCR_GadgetManagerComponent`, `BaseRadioComponent`,
  `BaseTransceiver`, `ChimeraWorld.GetRadioManager`, `SCR_HintManagerComponent` and
  `SCR_PopUpNotification`; the `radioPlan` and `net` definitions in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_FrameworkManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`
  (`TBD_RadioBridgeStub.OnStageChanged`);
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches `TBD_RadioComponent`.
- Rules: a side's nets never reach another side, and a request carries no faction; a tune is
  reported only after its read-back matches; the net RPC stays within the eight-parameter limit;
  the framework takes no partner radio mod as a dependency; lines added stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile, while tuning is checked in game.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the briefing screen design, with its Frequencies section
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — why radio uses the engine's own
  transceivers and no partner mod
