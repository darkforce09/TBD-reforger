# Systems/Loadouts

Character loadout assembly and inventory equipment routines.

### Roles & Responsibilities
- `TBD_LoadoutEquipComponent.c`: Game mode component coordinating character equipment passes during slot body materialization.
- `TBD_LoadoutEquipHelper.c`: Static low-level routines executing clothes fitting, weapon mounting, and ammunition/item placement into storage inventories.

### Call Flow & Contracts
Authority-only execution. Invoked directly by `Systems/Spawning/TBD_SpawnManager` when instantiating physical slot character entities.
