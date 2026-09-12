# Systems/Mission/Data

Data models and deserialization structs mirroring the compiled mission schema (`packages/tbd-schema`).

### Roles & Responsibilities
- `TBD_MissionSlotStruct.c`: Schema definitions for playable infantry slots, group hierarchies, and authored transforms.
- `TBD_MissionVehicleStruct.c`: Schema definitions for mission vehicles, initial crews, cargo inventory, and spawn coordinates.
- `TBD_MissionParams.c`: Structs mapping authored mission parameters, time limits, and flow rules.
- `TBD_EntityState.c` & `TBD_VehicleState.c`: State override structs for damaged components, fuel levels, and lock statuses.
- `TBD_GadgetFlags.c`: Bitmask helpers for optional equipment and gadget authorization flags.

### Call Flow & Contracts
Pure data contracts populated by `Systems/Mission/Loaders/` via `JsonLoadContext`. Consumed across server-side lifecycle systems (`Systems/Mission/Ingestion/`, `Systems/Spawning/`).
