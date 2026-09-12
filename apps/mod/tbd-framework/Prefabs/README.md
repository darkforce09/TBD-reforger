# Prefabs

Reusable Enfusion entity templates (`.et`) defining component composition for framework managers and controllers.

### Roles & Responsibilities
- `Prefabs/Systems/TBD_GameMode.et`: Root game mode entity prefab composing lifecycle managers (`TBD_FrameworkManager`, `TBD_SpawnManager`, `TBD_ObjectivesComponent`, `TBD_RadioComponent`).
- `Prefabs/Systems/TBD_PlayerController.et`: Custom player controller prefab hosting replicated client/server RPC endpoints.

### Call Flow & Contracts
Instantiated into the world layer upon mission launch. Hosts server authority components and binds client player controllers upon connection.
