# Round orchestrator

The framework manager: the game mode component that loads the deployed
[mission](/documentation_v2/glossary/g_to_m.md#mission), owns the round's stage machine, applies the
mission's pacing, weather and settings, runs the round clock and the elimination check, and keeps
the end banner and debrief board the post-game screens show.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/
└── TBD_FrameworkManager.c  TBD_FrameworkManager: the stage machine; TBD_MissionFlow: the flow block
```

## How it works

`TBD_FrameworkManager` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. `GetInstance()` and `IsFrameworkWorld()`
resolve it off the live game mode on every call, never from a static, because statics outlive a
world and a `load_mission` [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) restarts the
world in-process; every modded vanilla class in the addon asks `IsFrameworkWorld()` before it acts.

```text
OnPostInit ─▶ LOADING ── mission loaded and valid ──▶ roster settled ──▶ LOBBY
                             apply flow, weather,        (2 s, longer while
                             settings; registry;          loadouts settle)
                             materialize slot bodies
LOBBY ──▶ BRIEFING ──▶ SAFE_START ── countdown ──▶ LIVE ── win or time ──▶ END ──▶ DEBRIEF
      (admin: #tbd stage next | <STAGE>)                  (TBD_SafestartManager.GoLive)
```

- Loading: `OnPostInit` prints the component roll-call one frame later, and on the server enters
  `LOADING`, starts `TBD_MissionLoader.BeginLoad()` and polls each second. Once the mission is
  loaded and valid it applies `flow`, `environment.windDirDeg` and `settings`, loads the registry,
  materializes the [slot](/documentation_v2/glossary/n_to_z.md#slot) bodies, starts the
  [event](/documentation_v2/glossary/a_to_f.md#event) roster fetch, and enters `LOBBY` when the roster has
  settled (force-settled after 2 s) and the loadout settle is no longer pending.
- `SetStage` is the only way the stage changes. It refuses `SAFE_START` on a world without
  `TBD_SafestartManager`, and any stage `TBD_SpawnManager.StageRefusalFor` refuses, keeping the
  reason for `GetLastStageRefusal()`. A transition logs `[TBD][Stage] <FROM> -> <TO>` and
  `[TBD] Stage → <STAGE>`, then tells the radio stub, `TBD_SpawnManager`, `TBD_SafestartManager` and
  this machine's local UI, and runs the stage's hook: `LOBBY` refreshes the deployable mission
  list, `BRIEFING` announces the authored `flow.briefingSeconds` without advancing on it, `LIVE`
  arms the end checks, and `END` broadcasts the winner and reason.
- `TBD_MissionFlow` turns `flow` into answers, testing each field against the
  `TBD_MissionFlowStruct.ABSENT` sentinel: `safeStartSeconds` goes to
  `TBD_SafestartManager.AdminSetSeconds`; `timeLimitSeconds` arms a 1 Hz round clock at `LIVE`,
  only when `winConditions.endOn` also declares `time_limit` (`0` means no limit), with chat
  warnings from 30 minutes down; `jip` (`always`, the default, `until_safestart_end` or
  `disabled`) is answered by `AllowsJoinAtStage` for the join door in `TBD_SpawnManager`.
- The end checks tick every 2 s while `LIVE`: first `TBD_ObjectiveRegistry.EvaluateEndTriggers`,
  then `faction_eliminated`, which fires when at least two sides fielded players and only one
  still has a living one. The clock, the objective triggers and elimination all end the round
  through `SetStage(END)`.
- End and debrief: entering `END` fixes the winner and reason for the banner (inferred from the
  objective triggers and survivors when the caller named none, else `admin`) and packs the
  scoreboard from `TBD_ResultsReporter.FillScoreboard`; kills are credited per player while
  `LIVE`, team kills excluded. `LOADING` and `LOBBY` clear them. `NotifyLocalStageUI` opens or
  closes `TBD_EndScreen` and `TBD_DebriefScreen` and calls the local player controller's
  `TBD_OnStageChanged`, from `SetStage` on a listen host and from the replication hook on a
  client.
- `settings.nightVision` false strips night-vision gadgets from each spawned body 1.5 s after it
  spawns; `settings.spectatorPolicy` is replicated for the client spectator controller.
- `OnDelete` removes every call-queue entry the component armed, so no timer fires into the next
  world.

## Authority

- Server: mission load, the stage machine, flow, weather and settings, the round clock, the end
  checks, kill credit and the night-vision strip. `OnPostInit` returns before any of it on
  `RplMode.Client`, and the stage methods carry `@authority server`.
- Client: `OnStageReplicated` (`@authority client`) opens and closes the local screens; on a
  listen host `SetStage` calls the same `NotifyLocalStageUI`, since the hook never fires on the
  authority. A dedicated server has no workspace, so it opens nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties:
  - `m_Stage`, the current `TBD_EGameStage`, with the hook `OnStageReplicated`
    (`@replicated m_Stage`);
  - `m_sSpectatorPolicy` and `m_bNightVision`, the authored settings; `m_sEndWinner`,
    `m_sEndReason` and `m_sDebriefBoard`, the end banner and the packed scoreboard. These five
    replicate without a hook or a `@replicated` tag.

## Boundaries

- Depends on: `TBD_EGameStage` and `TBD_SafestartManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; `TBD_ObjectiveRegistry` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_MissionLoader`,
  `TBD_RosterLoader`, `TBD_SpawnManager` and `TBD_RadioBridgeStub` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Registry`, `TBD_Log` and `TBD_PlayerChat`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_ResultsReporter` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; `TBD_DeployableMissionList`,
  `TBD_BriefingService`, `TBD_EndScreen` and `TBD_DebriefScreen` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the `flow`, `settings`, `environment` and
  `winConditions` definitions in `contracts_v2/definitions/mission.schema.json`.
- Used by: nearly every script under `apps/mod/tbd-framework/Scripts/Game/TBD/` through
  `GetInstance()`, `GetStage()` or `IsFrameworkWorld()`; `TBD_SafestartManager`,
  `TBD_WinConditionEvaluator`, `TBD_TriggerRuntime` and `TBD_SpawnManager`, which call `SetStage`;
  `TBD_AdminService`, through `HandleAdminStageCommand`; `TBD_SpawnManager` and
  `TBD_MissionValidator`, through `TBD_MissionFlow`; `TBD_EndScreen`, `TBD_DebriefScreen` and the
  spectator controller, through the replicated fields; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the component.
- Rules: the stage changes only through `SetStage`, and a round ends only through
  `SetStage(END)`; a `flow` field's presence is tested against its sentinel, never with a null
  test, and an authored `0` stays distinct from an absent value; the roll-call line
  `[TBD] roll-call: …` lists every manager component of the game mode prefab, and
  `cargo xtask mod world-boot` fails when one reads `MISSING`; both stage lines keep their
  prefixes, `[TBD][Stage]` and `[TBD] Stage`, because `cargo xtask mod remote-logs` accepts either
  as the LOBBY proof and the staging runbook quotes the first; lines added
  to the script stay ASCII, and `cargo xtask mod compile` checks that it compiles.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the event loop and one life
- [End screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner the replicated winner and reason feed
- [Debrief specification](/documentation_v2/mod/tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md)
  — the DEBRIEF scoreboard the kill count and packed board feed
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the stage log
  lines of a healthy boot
