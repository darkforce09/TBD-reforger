# T-180.8 — Faction Library templates + squad vehicles

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.7 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Library authority:** [`t153_faction_library.md`](t153_faction_library.md) · schema `faction-library.schema.json` · API `/api/v1/factions`  
**Verify log:** `.ai/artifacts/t180_8_verify_log.md`

---

## Problem

Stitch toolbar: **Load Predefined ORBAT** + **APPLY TEMPLATE**. Operator: named factions (US 1980s) live under a side via T-153 — must Load/Apply/Save. Squads need **Add Vehicle** (Stitch yellow vehicle badge / add-vehicle control). Today mission graph has empty/weak `vehiclesById`; Faction Library already has `roles[]` + `vehicles[]` pool.

---

## Locked

| ID | Decision |
|----|----------|
| H-L1 | Template list = `GET /api/v1/factions` filtered `side == active tab` |
| H-L2 | **Apply** = **replace** all squads/slots/vehicles under that side (confirm dialog). Cancel = no mutation |
| H-L3 | **Save** = update selected library faction from current side graph; **Save as** = POST new name |
| H-L4 | Apply materialize: one squad (or N if we group — **default: one squad named after faction**, roles → slots in order; index 0..; **first role = leaderSlotId** unless a role tag/name is SL) |
| H-L5 | Role loadout from library copied onto slot embedded loadout |
| H-L6 | Library `vehicles[]` → mission vehicles attached to that squad (`vehicleIds` + `vehiclesById` rows) |
| H-L7 | Add Vehicle: requires **`add_vehicle`** from .2 (ABSENT pre-.2) + `attach_vehicle`; picker from library pool ∪ registry vehicle kinds; **map presence required** (glyph/position) — not dead button |
| H-L7b | Apply materialize MUST run in `map-engine-core` (pure) so H1–H4/H9 are `cargo test -p map-engine-core` — not FE-only |
| H-L8 | CIV library factions never appear in ORBAT template dropdown |

---

## Materialize algorithm (Apply)

```text
confirm replace active side S
delete all squads (and their slots/vehicles) under factions with key==S
ensure faction row key==S (name from library.doc.name)
create squad (name = library name or "Squad 1")
for i, role in enumerate(library.roles):
  add_slot(role.role, tag=role.tag, assetId=role.character, loadout=role.loadout JSON)
  index = i
set_leader(squad, first_slot)  # or slot whose role matches /Squad Leader/i if present
for v in library.vehicles:
  create vehicle entity + attach_vehicle(squad, vehicle_id)
  place at default offset near side spawn / map center if no position — document choice in verify log
refresh orbat_nodes + squad lines
```

**Save (inverse):**

```text
roles[] = each slot in squad order → { role, tag, character: assetId, loadout }
vehicles[] = attached vehicles → { vehicle: resource, label }
PUT /factions/:id { side, name, roles, vehicles }
```

---

## File map

| File | Change |
|------|--------|
| `crates/map-engine-core/src/mission/apply_faction.rs` (NEW) or `doc/store` helpers | Pure materialize/replace side from `FactionDoc` JSON — **Class-R testable in Rust** |
| `crates/map-engine-core/src/doc/store.rs` | `replace_side_orbat`, vehicle CRUD already from .2 |
| `apps/website/frontend/src/orbat_manager.rs` | Template select, Apply confirm, Save/Save as |
| `apps/website/frontend/src/faction_manager.rs` / client | Reuse list/create/update |
| `apps/website/frontend/src/editor_ops.rs` | Add Vehicle place/attach |
| Map GPU | Vehicle icon or reuse slot ring with distinct glyph — minimal visible presence |
| `apps/website/api` IT | Existing factions tests remain green; add apply round-trip if needed |

---

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **H1** | Apply doc with R roles ⇒ exactly R slots under side; squad count ≥ 1 | `apply_faction_library_counts` |
| **H2** | Apply sets `leaderSlotId` to first slot or SL-named role | `apply_faction_sets_leader` |
| **H3** | Apply copies loadout JSON onto slot when present | `apply_faction_copies_loadout` |
| **H4** | Apply with V vehicles ⇒ `vehicleIds.len()==V` | `apply_faction_vehicles` |
| **H5** | Cancel path: snapshot hash before/after unchanged | `apply_cancel_noop` (FE or core dry-run) |
| **H6** | Save → GET roles.len matches mission slot count for side | IT or API test |
| **H7** | Template dropdown excludes other sides + CIV | FE filter test |
| **H8** | Add Vehicle increases vehicleIds; not a no-op button | ops test |
| **H9** | Second Apply replaces (slot count becomes new R, not R_old+R_new) | `apply_faction_replace_not_merge` |

---

## Verify

```bash
cargo test -p map-engine-core apply_faction_
make test-it
make ci-local-leptos
```

## Manual

| ID | Check |
|----|-------|
| M-H1 | Apply a real library faction (or golden POST) → tree matches roles |
| M-H2 | Save as → appears in dropdown for that side |
| M-H3 | Add Vehicle → badge on squad + visible on map |
| M-H4 | Apply confirm cancel leaves tree unchanged |

---

## Claude Code prompt — T-180.8 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.8** — Templates Apply/Save (T-153) + squad vehicles.

═══ READ ═══
  t180_8_templates_vehicles.md · t153_faction_library.md · hub · handoff
  apps/website/frontend/src/faction_manager.rs · dto FactionDoc
  packages/tbd-schema/schema/faction-library.schema.json

═══ PROBLEM ═══
  Stitch Apply Template + Save + Add Vehicle. Materialize library → mission side
  (replace). Vehicles attach + map presence. Rust-pure apply for Class-R tests.

═══ LOCKED ═══
  - Replace not merge (H9)
  - Confirm before Apply
  - First/SL role = leader
  - CIV excluded from dropdown
  - No dead Add Vehicle

═══ DO ═══
  1. apply_faction Rust + tests H1–H4, H9
  2. UI Apply/Save/Save as + confirm
  3. Add Vehicle attach + map
  4. verify log · tag T-180.8

═══ DO NOT ═══
  Docs · merge-on-apply · skip map presence · CIV in ORBAT templates

═══ VERIFY ═══
  cargo test -p map-engine-core apply_faction_
  make test-it
  make ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.8 · Ready for T-180.9
```
