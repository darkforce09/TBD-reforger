# Systems Domain Hub (`Scripts/Game/TBD/Systems`)

The Systems domain contains the persistent background simulation mechanics, world tools, and mission ingestion pipeline for the TBD Framework.

---

## Architecture Overview

Systems are the "tools in the workshop"—they run in the 3D world, manage physical assets, perform spatial calculations, deserialize mission specifications, and manage entity lifecycles. They are intentionally decoupled from game mode rules: `Systems` do not decide who wins or loses; they provide state queries, data, and capabilities that `Gamemode` and `Session` consume.

```text
Systems/
├── Mission/                   <-- JSON data contracts, loaders, validators, weather & scatter
├── Spawning/                  <-- Character body materialization & dynamic respawns
├── Loadouts/                  <-- Dressing entities with weapons, gear, and uniforms
├── Audio/                     <-- 3D positional audio source entities & music cues
├── Zones/                     <-- 3D/2D spatial trigger volumes & boundary enforcement
├── Markers/                   <-- Tactical 2D map marker sync & local controllers
├── Radio/                     <-- Team radio frequency plans & external VOIP bridge
└── AI/                        <-- Waypoint routing & group AI state runtime
```

---

## Subdirectories

| Subdirectory | Responsibility | Key Classes |
|---|---|---|
| **`Mission/`** | Deserializes mission JSON into typed Enforce structs, validates schema rules, fetches rosters, and sets world weather/scatter. | `TBD_MissionLoader`, `TBD_MissionValidator`, `TBD_MissionSlotStruct`, `TBD_WeatherRuntime` |
| **`Spawning/`** | Translates mission `slots[]` into physical character entities in the world; handles player possession and respawn waves. | `TBD_SpawnManager`, `TBD_DynamicSpawner`, `TBD_SCR_RespawnSystemComponent` |
| **`Loadouts/`** | Reads JSON equipment blocks and equips clothing, armor, weapons, ammo, and items into inventory slots. | `TBD_LoadoutEquipComponent`, `TBD_LoadoutEquipHelper` |
| **`Audio/`** | Spawns positional `TBD_AudioSourceEntity` instances for ambient sound and fires event-driven music cues. | `TBD_AudioEmitter` |
| **`Zones/`** | Mathematical spatial geometry (circles, convex polygons) and high-performance entity overlap queries; enforces play area boundaries. | `TBD_PlayAreaComponent`, `TBD_TriggerRuntime`, `TBD_ZoneRegistry`, `TBD_ZoneVolume` |
| **`Markers/`** | Replicates user and mission tactical map markers across clients. | `TBD_MarkerComponent`, `TBD_MarkerController` |
| **`Radio/`** | Allocates radio frequency channels to squads and interfaces with external voice systems. | `TBD_RadioComponent`, `TBD_RadioController` |
| **`AI/`** | Executes waypoint paths and monitors AI group combat states. | `TBD_AIWaypointRuntime`, `TBD_AIGroupState` |
