# Mission flow

The reader of a [mission](/documentation/glossary/g_to_m.md#mission)'s `flow` block: how long the
briefing, the safe start and the round run, and who may still join a round in progress.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Flow/
├── TBD_EJipPolicy.c         TBD_EJipPolicy: always, until_safestart_end or disabled
├── TBD_JipPolicy.c          TBD_JipPolicy: resolves flow.jip and answers per stage whether a join is allowed
├── TBD_MissionFlow.c        TBD_MissionFlow: flow durations resolved to seconds or UNSET, end trigger names
└── TBD_MissionFlowReport.c  TBD_MissionFlowReport: puts flow into force at load and announces the briefing
```

## How it works

`TBD_MissionFlow` reads `flow` off the live mission document on every call and tests each field
against `TBD_MissionFlowStruct.ABSENT`, because the JSON reader allocates the block even when the key
is absent. `ResolveSeconds` turns a raw duration into seconds with a source label: `authored`,
`default` (absent, returned as `UNSET`) or `INVALID` (negative, also `UNSET`). An authored `0`
stays `0`; for `timeLimitSeconds` it means no limit.

`TBD_JipPolicy` maps `flow.jip` onto `TBD_EJipPolicy` (absent or unknown values are `ALWAYS`) and
answers `AllowsJoinAtStage`: every policy allows `LOADING` and `LOBBY`, `DISABLED` closes from
`BRIEFING`, `UNTIL_SAFESTART_END` closes from `LIVE`. The join door, `TBD_SpawnJoinAudit` in
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Identity/`, calls `AllowsJoinAtStage` and
`Name` directly.

`TBD_MissionFlowReport.Apply` runs once per mission load, from `TBD_LoadingGate` before the round
leaves `LOADING`: it hands `flow.safeStartSeconds` to `TBD_SafestartManager.AdminSetSeconds` (a
refused value is logged at ERROR, never clamped) and writes one `[TBD][Flow]` line per field naming
the source of the value in force. `AnnounceBriefing` runs on entering `BRIEFING` and tells players
the authored briefing length; nothing advances on it.

## Authority

- Server: `Apply`, the reports and the briefing announcement (`@authority server`).
- Client: nothing; a client holds no mission document, so every accessor answers absent.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` and `TBD_MissionFlowStruct` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_SafestartManager` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; `TBD_Log`, `TBD_PlayerChat` and
  `TBD_ClockText` under `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the `flow` definition in
  `contracts/definitions/mission.schema.json`.
- Used by: `TBD_LoadingGate`, `TBD_RoundClock`, `TBD_FactionElimination` and `TBD_EndBanner` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Stage/`; `TBD_FrameworkManager`;
  the win rule under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; the join door and
  the mission validator under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`.
- Rules: presence is tested against the sentinel, never with a null test; nothing is cached, so no
  value outlives an in-process world restart; the permitted-join label is derived from
  `AllowsJoinAtStage`; lines added to a script stay ASCII, and `cargo xtask mod compile` checks
  that the scripts compile.
