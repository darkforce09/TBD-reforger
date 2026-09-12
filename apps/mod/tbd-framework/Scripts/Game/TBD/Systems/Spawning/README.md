# Systems/Spawning

Authoritative slot materialization, player character binding, possession transitions, and one-life elimination.

### Roles & Responsibilities
- `TBD_SpawnManager.c`: Central authority materializing pre-spawned slot bodies from mission definitions and binding joining players.
- `TBD_DynamicSpawner.c`: World spawner instantiating mission vehicles, equipment crates, and mission entities.
- `TBD_SCR_MenuSpawnLogic.c`: Hooks into vanilla spawn logic to bypass default respawn screens in favor of TBD slot assignments.
- `TBD_SCR_PossessSpawnHandlerComponent.c`: Transitions client camera from the pre-slot overlook to the physical character body upon deployment.
- `TBD_SCR_RespawnSystemComponent.c`: Enforces one-life elimination rules, routing dead players directly into spectator mode.

### Call Flow & Contracts
Authority-only simulation system. Communicates with `Session/Lobby/` to register slot claims and assigns player controllers to spawned character entities upon round start.
