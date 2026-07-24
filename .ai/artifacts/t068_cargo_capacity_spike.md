# T-068 cargo capacity spike

**Date:** 2026-07-21 · **Status:** PINNED for T-068.15.1 export  
**Operator:** grids in scope; plugin auto-grab; **Fable 5 executes** (2026-07-24)

## Prefab capacity (SoT)

| Export field | Prefab / API |
|--------------|--------------|
| `max_weight_kg` | storage `m_fMaxWeight` (ancestry; resolve leaf override) |
| `max_volume_cm3` | storage `MaxCumulativeVolume` |
| ClothNode (ALICE) volume | **sum** of child `SCR_UniversalInventoryStorageComponent.GetMaxVolumeCapacity()` (matches UI `GetMaxVolumeCapacity`) |

## Cargo grid (PINNED)

`SCR_InventoryStorageBaseUI` defaults: **`m_iMaxColumns = 4`**, **`m_iMaxRows = 3`**, pageSize 12.

**Universal cloth containers (pants/jacket/backpack panels):**

```
VOLUME_PER_CELL_CM3 = 50
cargo_grid_w = 4
cells = ceil(max_volume_cm3 / 50)
cargo_grid_h = max(3, ceil(cells / 4))
```

Class-R vs operator screenshot:

| Prefab | vol | cells | grid | Screenshot |
|--------|-----|-------|------|------------|
| Jacket_US_BDU_base | 800 | 16 | **4×4** | BDU Blouse 4×4 |
| Pants_Trousers_01_base | 600 | 12 | **4×3** | BDU Trousers 4×3 |

`OpenedStorageUI` can take custom cols/rows (defaults 6×3) when opening nested containers — garment **panel** SoT remains BaseUI formula above.

LBS (ALICE) UI inherits BaseUI 4×3; nested pouch opens may differ. Export LBS parent grid from **summed child Universal volumes** with same ÷50 / width-4 formula (Class-R in verify).

## Default cargo (PINNED)

Source: `SCR_InventoryStorageManagerComponent` → `InitialInventoryItems[]`:

- `TargetStorage` — path string e.g. `Pants/Pants_US_BDU.et`, `Vest/.../MagPouch/...`
- `PrefabsToSpawn[]` — item ResourceNames (duplicates = qty)

US Rifleman / BaseLoadout confirmed: pants get radio/compass/map/medical/etool; vest pouches get STANAG mags + grenades/smokes.

### Compat edges

`character_default_cargo`: `from_node` = item, `to_node` = character, `evidence` = `TargetStorage=<path>` (qty via duplicate edges or evidence suffix `;qty=N` — prefer **one edge per spawn entry** so count = edge multiplicity, OR collapse in ingest).

**Export choice (locked):** emit **one edge per PrefabsToSpawn entry** (multiplicity = qty). Arsenal collapses by (container,item).

Container key for Arsenal: first path segment of TargetStorage (`Pants`→pants, `Jacket`→jacket, `Vest`→vest, `Back`→backpack).

## Tooling

- `game_read` OK for uncompressed `.et`; fails on compressionLevel=6 `.c`
- `wb_script_editor` OK (lineText mapping fixed in local enfusion-mcp dist)

## T-068.15.1 measured addendum (2026-07-24, Fable 5)

Ship-time measurements that refine the pins above (all values from `game_read`
prefab dumps + the landed export):

- **Jacket 800 confirmed at the source:** `Jacket_US_BDU_base.et` serializes
  `MaxCumulativeVolume 800` / `m_fMaxWeight 5` on its
  `SCR_UniversalInventoryStorageComponent` — the 4×4 screenshot parity holds.
  The earlier 1000 in the export was a **scanner sourcing bug**: the jacket chain
  also carries a nested `SCR_EquipmentStorageComponent` (flashlight slot) whose
  resolved `MaxCumulativeVolume` (1000) won `ReadPhysAttrs`' map-iteration order.
  Fixed by a two-pass read (Universal storages first, then the old any-storage
  fallback for crates/vehicle trunks) — `TBD_RegistryScan.c ReadPhysAttrsPass`.
- **`Pants_US_BDU` is genuinely 1600 cm³ / 10 kg** (serialized on its Universal
  storage in `Pants_US_BDU_base.et`) → grid 4×8. The 4×3 trousers screenshot is
  `Pants_Trousers_01` (600) — both correct; the table above stays as pinned.
- **`Vest_PASGT` has no storage capacity at all** (pure armor) — capacity fields
  legitimately absent per locked decision 3.
- Export census after fix: 1857 items, **1257 grids** (= every row with a
  readable `max_volume_cm3`), 20,908 edges = 4,685 legacy (histogram unchanged)
  + **16,223 `character_default_cargo`** (5,919 distinct after qty aggregation;
  max qty 40 — ammo-carrier backpacks).
- `wb_reload` "compilation triggered (ExecuteAction=false)" measurably does
  **not** recompile (re-export still ran old code); a Workbench restart does.
- `wb_script_editor` `getLine`/`getLinesCount` are **0-based** and report
  `disk lines + 1` (phantom last line): disk line N = SE line N−1.
