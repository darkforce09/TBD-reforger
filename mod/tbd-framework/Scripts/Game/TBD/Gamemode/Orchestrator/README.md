# Round orchestrator

The framework manager: the game mode component that loads the deployed
[mission](/documentation/glossary/g_to_m.md#mission), owns the round's stage machine, applies the
mission's pacing, weather and settings, runs the round clock and the elimination check, and keeps
the end banner and debrief board the post-game screens show.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/
├── Flow/                    the mission's `flow` block: durations, join policy, load report
├── Heartbeat/               TBD_RuntimeHeartbeat: the one game-mode loop that ticks the mission runtimes
├── Stage/                   the helpers the manager owns: loading gate, end checks, round clock, END banner
├── TBD_FrameworkManager.c   TBD_FrameworkManager: the stage machine and its replicated fields
└── TBD_FrameworkRollCall.c  TBD_FrameworkRollCall: the component roll-call line world-boot asserts
```

## How it works

`TBD_FrameworkManager` is a `SCR_BaseGameModeComponent` on
`mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. `GetInstance()` and `IsFrameworkWorld()`
resolve it off the live game mode on every call, never from a static, because statics outlive a
world and a `load_mission` [fleet command](/documentation/glossary/a_to_f.md#fleet-command) restarts the
world in-process; every modded vanilla class in the addon asks `IsFrameworkWorld()` before it acts.
The manager keeps the engine hooks, the six replicated fields and the public surface; its
constructor creates one instance of each helper in `Stage/`, and `OnDelete` cancels every
call-queue entry they armed.

```text
OnPostInit ─▶ LOADING ── mission loaded and valid ──▶ roster settled ──▶ LOBBY
                             place entities; apply       (2 s, longer while
                             flow, weather, settings;     loadouts settle)
                             registry; materialize
                             slot bodies
LOBBY ──▶ BRIEFING ──▶ SAFE_START ── countdown ──▶ LIVE ── win or time ──▶ END ──▶ DEBRIEF
      (admin: #tbd stage next | <STAGE>)                  (TBD_SafestartManager.GoLive)
```

- Loading: `OnPostInit` schedules `TBD_FrameworkRollCall` one frame later and, on the server,
  enters `LOADING`, starts `TBD_MissionLoader.BeginLoad()` and hands over to `TBD_LoadingGate`
  (`Stage/`), which applies the mission to the world on the main thread, the only place it does,
  and asks for `LOBBY` once the roster and loadouts settle.
- `SetStage` is the only way the stage changes. It refuses `SAFE_START` on a world without
  `TBD_SafestartManager`, and any stage `TBD_SpawnManager.StageRefusalFor` refuses, keeping the
  reason for `GetLastStageRefusal()`. A transition logs `[TBD][Stage] <FROM> -> <TO>` and
  `[TBD] Stage -> <STAGE>`, then tells the radio stub, `TBD_SpawnManager`, `TBD_SafestartManager`
  and this machine's local UI, and runs the stage's hook: `LOBBY` refreshes the deployable mission
  list, `BRIEFING` announces `flow.briefingSeconds` (`Flow/`), `LIVE` arms the end checks and the
  round clock (`Stage/`), and `END` broadcasts the winner and reason.
- Every end rule in this folder ends the round through `EndRound(reason, winner)`, which records
  the pair for the banner and calls `SetStage(END)`. Entering `END` fixes the replicated winner
  and reason (`TBD_EndBanner.Resolve`) and packs the scoreboard; `LOADING` and `LOBBY` clear them.
- `NotifyLocalStageUI` opens or closes `TBD_EndScreen` and `TBD_DebriefScreen` and calls the local
  player controller's `TBD_OnStageChanged`, from `SetStage` on a listen host and from the
  replication hook on a client.
- `LatchAuthoredSettings` copies `settings.spectatorPolicy` and `settings.nightVision` into the
  replicated fields; `TBD_StageEnvironment` strips night-vision gadgets on spawn when it is false.

## Authority

- Server: mission load, the stage machine, flow, weather and settings, the round clock, the end
  checks and the night-vision strip. `OnPostInit` returns before any of it on a
  client, and the stage methods and helpers carry `@authority server`.
- Client: `OnStageReplicated` (`@authority client`) opens and closes the local screens; on a
  listen host `SetStage` calls the same `NotifyLocalStageUI`, since the hook never fires on the
  authority. A dedicated server has no workspace, so it opens nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties, each tagged `@replicated`: `m_Stage`, the current `TBD_EGameStage`, with
  the hook `OnStageReplicated`; `m_sSpectatorPolicy` and `m_bNightVision`, the authored settings;
  `m_sEndWinner`, `m_sEndReason` and `m_sDebriefBoard`, the END banner and the packed scoreboard.

## Boundaries

- Depends on: `TBD_EGameStage` and `TBD_SafestartManager` under
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; `TBD_ObjectiveRegistry` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_MissionLoader`,
  `TBD_RosterLoader`, `TBD_SpawnManager` and `TBD_RadioBridgeStub` under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Registry`, `TBD_Log`, `TBD_PlayerChat`
  and `TBD_ClockText` under `mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_DeployableMissionList`,
  `TBD_BriefingService`, `TBD_EndScreen` and `TBD_DebriefScreen` under
  `mod/tbd-framework/Scripts/Game/TBD/Session/`; the `flow`, `settings`, `environment` and
  `winConditions` definitions in `contracts/definitions/mission.schema.json`.
- Used by: nearly every script under `mod/tbd-framework/Scripts/Game/TBD/` through
  `GetInstance()`, `GetStage()` or `IsFrameworkWorld()`; `TBD_SafestartManager`,
  `TBD_WinConditionEvaluator`, `TBD_TriggerRuntime` and `TBD_SpawnManager`, which call `SetStage`;
  `TBD_AdminService`, through `HandleAdminStageCommand`; `TBD_SpawnManager` and the mission
  validator, through `TBD_MissionFlow`; `TBD_EndScreen`, `TBD_DebriefScreen` and the
  spectator controller, through the replicated fields; and
  `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the component.
- Rules: the stage changes only through `SetStage`, and a round ends only through
  `SetStage(END)`; a `flow` field's presence is tested against its sentinel, never with a null
  test, and an authored `0` stays distinct from an absent value; the roll-call line
  `[TBD] roll-call: …` lists every manager component of the game mode prefab, and
  `cargo xtask mod world-boot` fails when one reads `MISSING`; both stage lines keep their
  prefixes, `[TBD][Stage]` and `[TBD] Stage`, because `cargo xtask mod remote-logs` accepts either
  as the LOBBY proof and the staging runbook quotes the first; lines added
  to the script stay ASCII, and `cargo xtask mod compile` checks that it compiles.

## Related documentation

- [Mod design](/documentation/mod/tbd-framework/mod_design.md) — the event loop and one life
- [End screen specification](/documentation/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner the replicated winner and reason feed
- [Debrief specification](/documentation/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the DEBRIEF scoreboard the packed board feeds
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the stage log
  lines of a healthy boot
