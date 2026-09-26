# Round rules and stage machine

The referee of a TBD round: the stage machine that carries it from loading to debrief, the safe
start before it goes live, the [mission](/documentation_v2/glossary/g_to_m.md#mission)'s objectives and
tasks, and every rule that ends it. The in-world machinery these rules read lives in
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/
├── Objectives/    capture, hold and destroy objectives, the objective end triggers, and tasks
├── Orchestrator/  the framework manager: mission load, the stage machine, round clock, end banner
└── Stages/        the stage enum, safe start, and the extraction and VIP win rules
```

## How it works

`TBD_FrameworkManager` in `Orchestrator/` owns the one stage value and the one way to change it,
`SetStage`. It loads the mission, enters `LOBBY` once the
[event](/documentation_v2/glossary/a_to_f.md#event) roster and the
[slot](/documentation_v2/glossary/n_to_z.md#slot) loadouts have settled, and from there an admin advances
the round (`#tbd stage next`), except that the safe start countdown in `Stages/` moves `SAFE_START`
to `LIVE` itself.

```text
LOADING ──▶ LOBBY ──▶ BRIEFING ──▶ SAFE_START ──▶ LIVE ──▶ END ──▶ DEBRIEF
            └──── safe start shield armed ────┘            │
                                   objectives advance ─────┘
```

Five rules can end a live round, and every one ends it through `SetStage(END)`:

| Rule | Where it is evaluated |
|---|---|
| `faction_eliminated`, `time_limit` | `Orchestrator/`, every 2 s and once a second |
| `all_objectives_captured`, `objective_destroyed`, `hold_expired` | `Objectives/`, asked by `Orchestrator/` every 2 s |
| `winConditions.mode` `extraction` and `vip` | `Stages/`, every 2 s |
| an editor trigger's `end_mission` effect | `TBD_TriggerRuntime` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/` |

Three game mode components live here, all on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`: `TBD_FrameworkManager`,
`TBD_SafestartManager` and `TBD_ObjectivesComponent`. The win rule and the task machine run from
their own `modded class SCR_BaseGameMode` ticks, fenced by
`TBD_FrameworkManager.IsFrameworkWorld()`. Every reader of the mission's JSON tests a block's
presence with a sentinel, because `JsonLoadContext` allocates a nested block even when its key is
absent, and every static is cleared when a new world starts, because statics outlive a world inside
one process.

## Authority

- Server: the stage machine, mission load, safe start, objectives, tasks and every end rule; each
  component or tick stops on `RplMode.Client` before arming, and clients hold no mission document.
- Client: the local screens that follow the replicated stage (`OnStageReplicated`), the safe start
  pop-ups (`OnCountdownReplicated`), and the once-a-second task HUD request.
- Owner: each player receives only their own side's objective board and capture bar.
- RPCs: none declared here; the objective and task HUD RPCs live in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`.
- Replicated properties: `TBD_FrameworkManager.m_Stage` (hook `OnStageReplicated`), the authored
  spectator and night-vision settings, the end winner, reason and debrief board; and
  `TBD_SafestartManager.m_iSecondsRemaining` (hook `OnCountdownReplicated`).

## Boundaries

- Depends on: the mission, spawning, zone and radio code under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Registry`, `TBD_Log` and
  `TBD_PlayerChat` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_ResultsReporter` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the end, debrief, briefing and mission-list code
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the HUD in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`; the mission shapes in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: nearly every script under `apps/mod/tbd-framework/Scripts/Game/TBD/`, through the stage
  and `IsFrameworkWorld()`; the admin commands in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`, which drive stages and safe start;
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the three components.
- Rules: the stage changes only through `TBD_FrameworkManager.SetStage`, and every end rule goes
  through it; a component the framework needs sits on `TBD_GameMode.et` and in the manager's
  roll-call, which `cargo xtask mod world-boot` checks; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the event loop, one life and the
  [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) facts the stage machine relies on
- [Mod UI screens](/documentation_v2/mod/tbd-framework/UI/README.md) — the screens each stage opens, the safe start,
  objective, end and debrief screens among them
