# Authored win rule

The runtime of a [mission](/documentation/glossary/g_to_m.md#mission)'s `winConditions.mode`: the
arm-time checks for every mode, and the evaluation of the two modes nothing else observes,
`extraction` and `vip`.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/WinConditions/
├── TBD_WinConditionEvaluator.c  TBD_WinConditionEvaluator: reads the rule, ticks it, ends the round once
├── TBD_WinConditionModes.c      TBD_WinConditionModes: mode names, arm-time warnings, extraction and VIP checks
└── TBD_WinConditionsStruct.c    TBD_WinConditionsStruct: the winConditions keys read; TBD_WinConditionDocStruct
```

## How it works

`TBD_RuntimeHeartbeat` calls `TBD_WinConditionEvaluator.Clear` on the way into a framework world
and `Tick` every 2 s (`TICK_MS`) until `HasEnded`. The first `Read` parses `winConditions` with its
own `TBD_MissionJsonPass` read into `TBD_WinConditionsStruct`; `endOn` is left to
`TBD_MissionLoader.HasEndTrigger`.

At the first `LIVE` tick `TBD_WinConditionModes.ReportRule` logs the mode once and warns when the
mission cannot satisfy it: `objective` without an objective trigger in `endOn`, `timeout` without
`time_limit`, an `extraction` or `vip` zone id that names no usable zone. `attrition`, `objective`
and `timeout` then end through `TBD_FactionElimination`, `TBD_ObjectiveRegistry` and
`TBD_RoundClock`. `extraction` ends the round when every living player of one side stands in the
`extractionZoneId` zone (a side with no living player does not count; a zone naming a faction
extracts only it); `vip` ends it when the `vipSlotId` player dies (`vip_down`, the one other
fielded side wins) or, with an extraction zone, reaches it (`vip_extracted`). The `s_bEnded` latch
is set before `SetStage(END)`, so a condition that stays true ends the round once.

## Authority

- Server: everything; `Tick` carries `@authority server`, and a client holds no mission document,
  so `Read` finds no rule there.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_FrameworkManager` (the stage and `SetStage`) and `TBD_MissionFlow` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`; `TBD_ObjectiveRegistry`'s
  trigger names under `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`;
  `TBD_SpawnManager`, `TBD_MissionLoader`, `TBD_MissionJsonPass` and `TBD_ZoneRegistry` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Log` and `TBD_AnnounceOnce` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; the `winConditions` definition in
  `contracts/definitions/mission.schema.json`.
- Used by: `TBD_RuntimeHeartbeat` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/Heartbeat/`.
- Rules: the latch and the rule clear on the way into a world, because statics outlive a world; no
  second round timer is started for `timeout`; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [End screen specification](/documentation/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner that names the win rule's endings
