# Systems/AI

Runtime simulation, behavioral state, and waypoint navigation for non-player AI groups.

### Roles & Responsibilities
- `TBD_GroupState.c`: Manages AI group runtime parameters, tactical stances, combat formations, and movement speeds.
- `TBD_WaypointRuntime.c`: Evaluates authored waypoint chains, driving group movement, patrols, and defensive holds.

### Call Flow & Contracts
Authority-only simulation. Reads group and waypoint definitions from `Mission/Data/` and controls native `SCR_AIGroup` entities.
