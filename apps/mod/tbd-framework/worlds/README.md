# worlds

Enfusion world entities (`.ent`), subscenes, and layer entity placement data.

### Roles & Responsibilities
- `TBD_Dev_POC.ent`: Root world scene referencing the base Eden terrain (`worlds/Eden/Eden.ent`) as a parent subscene.
- `TBD_Dev_POC_Layers/default.layer`: World layer entity instantiating the `TBD_GameMode.et` prefab at world coordinates `(6400, 0, 6400)`.
- `Eden/`: Editor metadata and user map descriptors for the Eden island subscene.

### Call Flow & Contracts
Loaded by Enfusion when launching scenario configurations from `Missions/`. Instantiates the world environment and spawns layer entities on boot.
