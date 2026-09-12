# Systems/Mission/Loaders

Document fetchers, deserializers, and validation passes for compiled mission files and event rosters.

### Roles & Responsibilities
- `TBD_MissionLoader.c`: Primary mission loader; fetches compiled JSON via REST endpoint or `$profile:missions/` fallback and parses into memory.
- `TBD_MissionValidator.c`: Fail-fast validator running immediately post-parse (blocks stage progression on fatal structural errors; emits diagnostics on warnings).
- `TBD_MissionListLoader.c`: Queries backend for available missions catalog for administrative scenario selection.
- `TBD_RosterLoader.c`: Ingests web ORBAT assignments mapping player identities to pre-assigned slot IDs.

### Call Flow & Contracts
Authority-only execution during the `LOADING` stage. Invoked by `Gamemode/Orchestrator/TBD_FrameworkManager`; on success, hands populated mission structures to game mode components and simulation systems.
