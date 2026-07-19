# T-180 — Class-R pin ledger (measured 2026-07-19)

**Authority companion to** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md).  
Every numeric/RGBA/package/API claim below was measured in this checkout. Slice specs must not contradict this file.

---

## Cargo packages (`cargo test -p …`)

| Crate path | Package name | Use |
|------------|--------------|-----|
| `crates/map-engine-core` | **`map-engine-core`** | Graph, orbat derive, tint, squad_links, apply_faction |
| `apps/website/frontend` | **`website-frontend`** | Place path, dock, ORBAT UI (NOT `website-leptos`) |
| `apps/website/api` | **`website-api`** | `make test-it` / factions / events |

False-green: any VERIFY block saying `website-leptos` is **wrong**.

**Doc feature gate (T-180.1 measured):** `MissionDocCore` tests need  
`cargo test -p map-engine-core --features doc <filter>`  
Bare `cargo test -p map-engine-core <filter>` can match **0** tests.

---

## Exact colors (map rings + lines)

From [`slots_gpu.rs:22-24`](../../../crates/map-engine-core/src/slots_gpu.rs) + [`aegis.css:17,64,71`](../../../apps/website/frontend/style/aegis.css):

| Token | Hex | RGBA u8 (locked for T-180.3/4) |
|-------|-----|--------------------------------|
| BLUFOR / current primary | `#adc6ff` | **`[173, 198, 255, 255]`** (= `SLOT_PRIMARY_RGBA`) |
| OPFOR / error-alert | `#f87171` | **`[248, 113, 113, 255]`** |
| INDFOR / success | `#22c55e` | **`[34, 197, 94, 255]`** |
| Selected | `#facc15` | **`[250, 204, 21, 255]`** (= `SLOT_SELECTED_RGBA`) |

Gate: `assert_ne!` all three side arrays pairwise; selected path unchanged.

---

## Graph keys today (store mutators)

| Entity | File:line | Keys written |
|--------|-----------|--------------|
| Slot | `store.rs:275-307` | `id,squadId,index,role,tag?,assetId?,position{x,y,z,rotation},stance,loadoutId` — **no callsign/rank** |
| Faction | `store.rs:311-318` | `id,key,name,squadIds` |
| Squad | `store.rs:324-336` | `id,factionId,name,slotIds,callsign?` — **no leaderSlotId, no vehicleIds** |
| Loadout | `store.rs:408+` | `update_slot_loadout` → slot.`loadout` object incl. `summary` |
| Layer refile only | `store.rs:853` | `move_slot_to_layer` — **≠** squad refile |
| Vehicles map | `store.rs:169,752,780` | `vehiclesById` hydrate exists — **`add_vehicle` ABSENT** |

**ABSENT (must be added by slices):** `leaderSlotId`, `set_leader`, `move_slot_to_squad`, `rename_squad`, `vehicleIds`, `add_vehicle`, slot `callsign`/`rank`.

---

## Place path today

| Item | File:line |
|------|-----------|
| Defaults | `editor_ops.rs:41-48` — `faction-1` / `Faction 1` / `squad-1` / `Squad 1` |
| `ensure_default_squad` | `editor_ops.rs:903-920` |
| `place_at` | `editor_ops.rs:929-960` — always `ensure_default_squad`; `index: 0` |
| `OpsCtx` | `editor_ops.rs:50-71` — has `orbat_nodes`, `attrs_open`; **no `active_side`** |
| Signals | `mission_editor.rs:112-118` — `orbat_nodes`, `attrs_open`; **no `active_side`** |

### Locked place model (T-180.1)

```text
Faction rows: STABLE one per side
  id = "faction-BLUFOR" | "faction-OPFOR" | "faction-INDFOR"
  key = "BLUFOR" | "OPFOR" | "INDFOR"
  name = same as key (or display name later)

Each place_at:
  1. active_side ∈ {BLUFOR,OPFOR,INDFOR} (default BLUFOR until chips)
  2. ensure faction row for that side (idempotent)
  3. mint NEW squad id (never reuse squad-1 as singleton dump)
  4. add_slot; set squad.leaderSlotId = new slot id
  5. index = 0 on sole member
```

**Testability:** extract pure helper into `map-engine-core` (e.g. `mission/place_orbat.rs` or store method) so A1/A4 are `cargo test -p map-engine-core`. `place_at` only calls helper + layer/selection. FE may add a thin wasm/ops test but **core tests are mandatory**.

---

## Faction `key` drop bug (must fix ≤ T-180.1 or .5)

| Site | Behavior |
|------|----------|
| `editor_ops.rs:668-676` `faction_rows` | Reads `id`,`name`,`squadIds` — **drops `key`** |
| `outliner.rs:169-174` `FactionRow` | **no `key` field** |
| `orbat.rs:119` derive | Uses `f.key` for Event ORBAT faction string |

**Pin:** `FactionRow` must gain `key: String`. Side filter, tint, and ORBAT tabs use **`key`**, not `id`/`name`. Gate: unit test `faction_rows_preserves_key`.

---

## Attributes / Open Arsenal (T-180.9)

| Item | File:line |
|------|-----------|
| Open | `editor_ops.rs:454-471` `open_attributes(id)` → `attrs_open.set(Some(id))` |
| Tabs | `attributes.rs:16` `TABS = ["Transform","Identity","States","Arsenal"]` → Arsenal index **3** |
| Tab signal | `attributes.rs:43` `let tab = RwSignal::new(1usize)` — **defaults Identity (1)**; local to modal |
| Arsenal mount | `attributes.rs:145` area + `arsenal.rs:247` → `set_loadout` |

**Pin:** Add `open_arsenal(id: String)` that opens Attributes **and** selects tab **3**. Gate fails if only `open_attributes` (lands on Identity).

---

## ORBAT derive / Event / Export

| Item | File:line |
|------|-----------|
| `OrbatSlotTemplate` | `orbat.rs:11-17` — serde **snake_case** `role/loadout/tag` |
| Derive hardcode | `orbat.rs:114` `loadout: String::new()` |
| `Sl` struct | `orbat.rs:79-85` — **no loadout field** |
| Empty assert | `orbat.rs:174` `all loadout.is_empty()` — **must invert** |
| Export inject | `compile.rs:103-109` |
| Export empty fixture | `compile.rs:248-254` `"loadout": ""` — update when filling |
| Event materialize | `events.rs:82-83` `.bind(&sl.loadout)` → DB column `loadout` |
| Summary join | `arsenal.rs:201-206` keys primary/optic/magazine/launcher `.join(" · ")` |

**Fill pin:** prefer `loadout.summary`; else rebuild; Event/Export see non-empty string.

---

## Faction Library API (T-180.8)

| Item | File:line |
|------|-----------|
| Routes | `api/src/app.rs:77,81` `/factions`, `/factions/{id}` |
| Handlers | `handlers/factions.rs:44,64,84,110,153` |
| DTO | `dto.rs:349-385` `FactionRole` / `FactionVehicle` / `FactionDoc` / `UserFaction` |

---

## Hairline upload (T-180.4)

| Item | File:line |
|------|-----------|
| LineList upload | `map-engine-render/src/engine.rs` ~4399 (hairline segments `[x,y,r,g,b,a]…`) |

---

## Stitch assets

`.ai/artifacts/t180_stitch_orbat_modal/{screen.png,code.html,DESIGN.md,eden_side_chips_ref.png}`

---

## Cross-slice dependency (hard)

```text
.1 helper+leaderSlotId+key+active_side
 → .2 mutators/GC/vehicleIds
 → .3 tint (needs key→side)
 → .4 lines (needs leader+xy+side rgba)
 → .5 chips (writes active_side)
 → .6 refile sync
 → .7 Stitch UI
 → .8 apply/save/add_vehicle
 → .9 derive fill + open_arsenal(tab=3)
```

Do not start N+1 without N verify log PASS quoting the gate commands below.
