# Round stages, safe start and authored win rules

The stage vocabulary of a TBD round, the safe start that keeps everyone unhurt from the lobby until
the round goes live, and the evaluator that ends the round on a
[mission](/documentation/glossary/g_to_m.md#mission)'s authored `extraction` or `vip` win rule.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/
├── Safestart/         game mode component: damage off, rounds deleted, countdown to LIVE, verified lift
├── WinConditions/     winConditions.mode: ends the round on extraction or a VIP's fate
└── TBD_EGameStage.c   TBD_EGameStage: the seven round stages, LOADING to DEBRIEF
```

## How it works

`TBD_EGameStage` is the one stage enum every folder reads; its order is the round order and the
admin `#tbd stage next` order. `TBD_FrameworkManager` in
`mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/` owns the current stage and calls
`TBD_SafestartManager.OnStageChanged` on every transition.

- [`Safestart/`](Safestart/README.md): the shield arms on `LOBBY`, `BRIEFING` and `SAFE_START`,
  counts down to `LIVE` on `SAFE_START`, and lifts on any other stage, handing each body back the
  damage value it had.
- [`WinConditions/`](WinConditions/README.md): the authored `winConditions.mode`; `extraction` and
  `vip` end the round here, the other modes end through the orchestrator and the objectives.

## Authority

- Server: the shield, the countdown, the lift and the win rule.
- Client: the safe start pop-ups (`OnCountdownReplicated`).
- Owner: nothing.
- RPCs: none.
- Replicated properties: `TBD_SafestartManager.m_iSecondsRemaining`, with the hook
  `OnCountdownReplicated`.

## Boundaries

- Depends on: `TBD_FrameworkManager` (the stage and `SetStage`); the spawning, mission and zone code
  under `mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Log`, `TBD_PlayerChat` and
  `TBD_ClockText` under `mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: `TBD_FrameworkManager`; `TBD_RuntimeHeartbeat`, which ticks the win rule;
  `TBD_AdminService` in `mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`
  (`#tbd safestart status|go|<seconds>`); `TBD_EGameStage`, read across `API/`, `Session/`,
  `Systems/` and `UI/` under `mod/tbd-framework/Scripts/Game/TBD/`; and
  `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches `TBD_SafestartManager`.
- Rules: a new stage goes where it runs in the round; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Safe start HUD specification](/documentation/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md)
  — the safe start notices as built and their design target
- [End screen specification](/documentation/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner that names the win rule's endings
- [Mod design](/documentation/mod/tbd-framework/mod_design.md) — one life and the
  [event](/documentation/glossary/a_to_f.md#event) loop the stages follow
