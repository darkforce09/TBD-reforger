# Gamemode/Objectives

Dynamic mission objective evaluation, capture points, task lifecycles, and win trigger hooks.

### Roles & Responsibilities
- `TBD_ObjectivesComponent.c`: `SCR_BaseGameModeComponent` ticking at 1 Hz on the server during `LIVE`. Evaluates player presence inside zones, advances progress, and replicates state to client HUDs.
- `TBD_ObjectiveRegistry.c`: Central authoritative registry caching all active mission objectives, task states, and win conditions.
- `TBD_Objective.c`: Runtime model encapsulating individual objective parameters, progress counters, and team ownership.
- `TBD_ObjectiveRules.c`: Domain rules for territory capture, target destruction, and hold mechanics.
- `TBD_TaskStateMachine.c`: Formal state machine tracking task lifecycle transitions (`INACTIVE`, `ACTIVE`, `CONTESTED`, `COMPLETED`, `FAILED`).

### Call Flow & Contracts
1. **Tool Ingestion**: Queries `Systems/Zones/` to determine which players occupy objective zones.
2. **Win Condition Hooks**: Objective completion states feed directly into `Gamemode/Stages/TBD_WinConditionEvaluator` to inform `Gamemode/Orchestrator/TBD_FrameworkManager` of match victory or defeat.
3. **Client HUD**: Pushes diff-hashed updates to clients via targeted RPCs to drive `UI/layouts/Hud/TBD_ObjectiveHud.layout`.
