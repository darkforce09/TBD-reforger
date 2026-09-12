# Session/MissionSelector

Scenario selection and terrain rotation interface for server administrators.

### Roles & Responsibilities
- `TBD_MissionBrowser.c`: `modded class SCR_PlayerController` providing client<->server RPC transports for admins to query mission lists, cycle scenarios, and initiate loads.
- `TBD_ScenarioRouter.c`: Maps mission terrain slugs (e.g. `"everon"`) to their corresponding `.conf` scenario headers and manages addon GUID transitions.
- `UI/TBD_MissionSelectorScreen.c`: Interactive screen controller allowing admins to browse available terrains, inspect mission details, and initiate scenario changes.

### Call Flow & Contracts
Admin selects a mission in `UI/TBD_MissionSelectorScreen` -> requests load via `TBD_MissionBrowser` RPC on `SCR_PlayerController` -> server authority triggers world change via `TBD_ScenarioRouter` or restarts scenario.
