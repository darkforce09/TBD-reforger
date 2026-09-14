# Systems/Loadouts

Character loadout assembly and inventory equipment routines.

### Roles & Responsibilities
- `TBD_LoadoutEquipComponent.c`: Game mode component coordinating character equipment passes during slot body materialization.
- `TBD_LoadoutEquipHelper.c`: Static low-level routines executing clothes fitting, weapon mounting, and ammunition/item placement into storage inventories.
- `TBD_LoadoutPreviewDresser.c` (2026-09-13): CLIENT-side twin of `TBD_LoadoutApplication` for the lobby kit preview — dresses the `ItemPreviewManagerEntity` preview entity of a kit prefab with a `TBD_SlotLoadoutStruct` (authored slot replaces, absent slot keeps the kit's own via a per-prefab BASELINE recorded on first resolve — the manager caches one entity per prefab and `GetSlotTemplate()` is empty for baked weapons — a baseline "empty" clears the slot; optic + magazine mount through `AttachmentSlotComponent.CanSetAttachment`, magazine falls back to the inventory manager; primary in hands). Local spawns into the preview world, synchronous `AttachEntity`, WARNING-once on misses; never replicates, never ERRORs. Not a subclass on purpose (header says why). Spawns vanilla's `ItemPreviewManager.et` locally when the world has none.

### Call Flow & Contracts
Equip component + helper: authority-only execution, invoked directly by `Systems/Spawning/TBD_SpawnManager` when instantiating physical slot character entities. The preview dresser is the one client-side file here: menu code (`Session/Lobby/UI/TBD_KitPreviewComponent`) calls it with the same kit alias + loadout shape.
