# Gamemode Domain Hub (`Scripts/Game/TBD/Gamemode`)

The Gamemode domain defines the dynamic match rules, round state progression, victory/defeat evaluation, and the central match orchestrator for the TBD Framework.

---

## Architecture Overview

Unlike arcade games with rigid, hardcoded game modes (e.g. Team Deathmatch), TBD missions are dynamically composed of modular objectives (Sector Control, HVT, Destroy Cache, VIP Extraction, Hold Areas). The Gamemode domain acts as the **conductor and referee** of the match: it defines how objectives work, runs the stage clock, and queries underlying world `Systems/` to evaluate win conditions.

```text
Gamemode/
├── Orchestrator/              <-- Central match conductor (TBD_FrameworkManager.c)
├── Stages/                    <-- Safestart, round stages, win condition evaluator
└── Objectives/                <-- Dynamic task state machines, objective registry, and rules
```

---

## Subdirectories

| Subdirectory | Responsibility | Key Classes |
|---|---|---|
| **`Orchestrator/`** | The central `SCR_BaseGameModeComponent` managing stage transitions, round clock, flow parameters (`doc.flow`), and debrief snapshots. | `TBD_FrameworkManager` |
| **`Stages/`** | Defines match phases (`LOADING` &rarr; `LOBBY` &rarr; `BRIEFING` &rarr; `LIVE` &rarr; `END` &rarr; `DEBRIEF`), safestart damage prevention, and win condition checks. | `TBD_GameStage`, `TBD_SafestartManager`, `TBD_WinConditionEvaluator` |
| **`Objectives/`** | Evaluates modular mission goals, advances capture/hold/destroy progress, and replicates state to client HUDs. | `TBD_ObjectivesComponent`, `TBD_ObjectiveRegistry`, `TBD_TaskStateMachine`, `TBD_ObjectiveRules` |

---

## Rules of Engagement

1. **Gamemode Defines Rules, Systems Provide Tools:**
   Gamemode components do not implement raw spatial geometry or entity creation. When an objective needs to know who is inside a capture zone, it queries `Systems/Zones/`. When a round starts, the Orchestrator commands `Systems/Spawning/` to materialize player slots.
2. **Authoritative Evaluation:**
   All objective advancement, stage transitions, and win condition decisions run server-authoritatively and are replicated to clients via state snapshots or targeted RPCs.
