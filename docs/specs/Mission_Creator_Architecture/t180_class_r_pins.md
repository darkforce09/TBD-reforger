# T-180 — Class-R pin ledger

**Authority companion to** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md).  
**Wins on conflict** with older slice prose. Coherency gate: `cargo xtask verify t180`.

---

## Cargo packages (`cargo test -p …`)

| Crate path | Package name | Use |
|------------|--------------|-----|
| `crates/map-engine-core` | **`map-engine-core`** | Graph, orbat derive, tint, squad_links, apply_faction |
| `apps/website/frontend` | **`website-frontend`** | Place path, dock, ORBAT UI (**NOT** `website-leptos`; bin tests) |
| `apps/website/api` | **`website-api`** | `cargo xtask db test-it` / factions / events |
| `crates/map-engine-render` | **`map-engine-render`** | Lane order / hairline / vehicles_bind |

**Feature gates:**
- Doc mutators / place / apply: `cargo test -p map-engine-core --features doc …`
- Derive / compile orbat: `cargo test -p map-engine-core --features mission …`
- Bare filter can match **0** tests if the wrong feature is omitted.

---

## Exact colors (map rings + lines)

| Token | Hex | RGBA u8 |
|-------|-----|---------|
| BLUFOR | `#adc6ff` | **`[173, 198, 255, 255]`** |
| OPFOR | `#f87171` | **`[248, 113, 113, 255]`** |
| INDFOR | `#22c55e` | **`[34, 197, 94, 255]`** |
| Selected | `#facc15` | **`[250, 204, 21, 255]`** |

Live: `crates/map-engine-core/src/slots_gpu.rs` `SIDE_*_RGBA`.

---

## Post-ship SoT (T-180.1–.9) — live measured

| Concept | Path / API |
|---------|------------|
| Place under side | `crates/map-engine-core/src/doc/place_orbat.rs` `place_character_under_side` |
| Graph mutators | `store.rs` — `set_leader`, `move_slot_to_squad` (+ empty GC), `add_vehicle` / attach |
| Apply template REPLACE | `crates/map-engine-core/src/doc/apply_faction.rs` `apply_faction_library` |
| Squad links geometry | `crates/map-engine-core/src/squad_links.rs` |
| Slot line UI string | `crates/map-engine-core/src/slot_line.rs` `format_slot_line` |
| Derive loadout summary | `crates/map-engine-core/src/mission/orbat.rs` `loadout_summary_from_value` |
| GPU upload | `apps/website/frontend/src/mission_history.rs` — `upload_squad_links` (role 9) + `vehicles_bind` |
| Eden chips | `eden_chrome.rs` `EDEN_SIDE_CHIPS = BLUFOR/OPFOR/INDFOR/Objects` |
| Refile | `editor_ops.rs` `refile_slot` → core `move_slot_to_squad` only |
| ORBAT Manager | `orbat_manager.rs` (thin re-export from `eden_chrome`) |
| Open Arsenal | `editor_ops.rs` `open_arsenal` → `attrs_open` + `attrs_tab = 3` |
| Placement default | Everon center `(6400, 6400)` |

**Faction keys:** stable `faction-BLUFOR|OPFOR|INDFOR`; filter/tint/tabs use **`key`**, not name substring.

**Place model:**
```text
active_side ∈ {BLUFOR,OPFOR,INDFOR}
→ ensure faction row for side
→ mint NEW squad; sole member = leaderSlotId
→ never dump into a singleton squad-1
```

---

## Pre-ship baseline (2026-07-19) — historical only

> Measured **before** T-180.1. Kept so older verify logs / slice prose remain interpretable.  
> **Do not** treat as live SoT.

| Item | Pre-ship claim |
|------|----------------|
| Squad | no `leaderSlotId` / `vehicleIds` |
| Slot | no callsign/rank |
| Place | `ensure_default_squad` → `faction-1` / `squad-1` |
| OpsCtx | no `active_side` |
| Derive | `loadout: String::new()`; `Sl` without loadout; assert all empty |
| Open Arsenal | `open_attributes` only → Identity tab |

---

## Faction Library API (T-180.8)

| Item | Path |
|------|------|
| Routes | `apps/website/api` `/api/v1/factions` |
| Schema | `packages/tbd-schema/schema/faction-library.schema.json` |

---

## Stitch assets

`.ai/artifacts/t180_stitch_orbat_modal/{screen.png,code.html,DESIGN.md,eden_side_chips_ref.png}`

---

## Cross-slice dependency (hard)

```text
.1 place+leaderSlotId+key+active_side
 → .2 mutators/GC/vehicleIds
 → .3 tint
 → .4 lines
 → .5 chips
 → .6 refile sync
 → .7 Stitch UI
 → .8 apply/save/add_vehicle
 → .9 derive fill + open_arsenal(tab=3)
 → .10 coherency gate (cargo xtask verify t180)
```

---

## Permanent gate

```bash
cargo xtask verify t180
```
