# T-180.8 verify log — Templates Apply/Save + squad vehicles

**Date:** 2026-07-19  
**Slice:** [`t180_8_templates_vehicles.md`](../../docs/specs/Mission_Creator_Architecture/t180_8_templates_vehicles.md)  
**Tag:** `T-180.8`

## Shipped

- New [`crates/map-engine-core/src/doc/apply_faction.rs`](../../crates/map-engine-core/src/doc/apply_faction.rs) — `apply_faction_library` REPLACE materialize (H-L7b)
- `MissionDocCore::set_faction_name` + `vehicle_xy_flat`
- `slots_gpu::pack_vehicle_instances` (disc + tactical yellow)
- `LaneRole::MissionVehicles` + `RenderEngine::vehicles_bind`; host bind in `mission_history` after_doc_change / rebind
- ORBAT Manager: template select (side filter, no CIV), Apply confirm, Save / Save as, Add Vehicle picker
- `editor_ops`: `orbat_apply_faction` / `orbat_add_vehicle` / `faction_doc_from_side`

**Placement pin:** Apply + Add Vehicle default to Everon center `(6400, 6400)` (= `INITIAL_TARGET`); slots `+15·i` in X; vehicles `+(30+20·j, −30)`.

## Gates

### H1–H4 / H9 — core apply (`--features doc`)

```text
$ cargo test -p map-engine-core --features doc apply_faction_
test doc::apply_faction::tests::apply_faction_copies_loadout ... ok
test doc::apply_faction::tests::apply_faction_vehicles ... ok
test doc::apply_faction::tests::apply_faction_library_counts ... ok
test doc::apply_faction::tests::apply_faction_sets_leader ... ok
test doc::apply_faction::tests::apply_faction_replace_not_merge ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 144 filtered out
```

### H8 pack + lane order

```text
$ cargo test -p map-engine-core pack_vehicle_instances
test slots_gpu::tests::pack_vehicle_instances_disc_yellow ... ok

$ cargo test -p map-engine-render mission_vehicles
test draw_order::lane_order_pins::mission_vehicles_sit_between_squad_links_and_slots ... ok
```

### H5 / H6 / H7 / H8 — FE unit

```text
$ cargo test -p website-frontend
test orbat_manager::tests::apply_cancel_noop ... ok
test orbat_manager::tests::save_faction_roles_match_side ... ok
test orbat_manager::tests::template_options_exclude_civ_and_other_sides ... ok
test orbat_manager::tests::orbat_add_vehicle_increases_vehicle_ids ... ok
test result: ok. 88 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### `make test-it`

PASS (factions CRUD suite remains green).

### `make ci-local-leptos`

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS (warnings only)
cargo test -p website-frontend                 PASS (88)
trunk build --release                          PASS
  (ambient NO_COLOR=1 → trunk `--no-color` quirk;
   `env -u NO_COLOR -u FORCE_COLOR make ci-local-leptos` ✅)
```

## Manual (operator)

| ID | Check |
|----|-------|
| M-H1 | Apply a library faction → tree matches roles |
| M-H2 | Save as → appears in dropdown for that side |
| M-H3 | Add Vehicle → badge on squad + yellow disc on map |
| M-H4 | Apply confirm cancel leaves tree unchanged |

## Ready for

**T-180.9** — Arsenal tab-3 + derive loadout fill
