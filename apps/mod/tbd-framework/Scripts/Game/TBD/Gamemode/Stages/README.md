# Gamemode/Stages

Match lifecycle state machine, safestart protection, and win condition evaluation.

### Roles & Responsibilities
- `TBD_GameStage.c`: Defines the game stage progression enum (`LOADING` through `DEBRIEF`) and manages transition events.
- `TBD_SafestartManager.c`: Enforces weapon safety, godmode invulnerability, and vehicle movement locks during pre-game stages.
- `TBD_WinConditionEvaluator.c`: Periodically evaluates round termination criteria (team attrition, captured objectives, time limits).

### Call Flow & Contracts
Authority-driven state machine. Stage changes update `TBD_FrameworkManager`, replicate to clients, and trigger transition events in `Session/` screens and safestart managers.
