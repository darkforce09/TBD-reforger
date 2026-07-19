# T-180.7 verify log — Stitch ORBAT Manager UI

**Date:** 2026-07-19  
**Slice:** [`t180_7_orbat_manager_ui.md`](../../docs/specs/Mission_Creator_Architecture/t180_7_orbat_manager_ui.md)  
**Tag:** `T-180.7`

## Shipped

- New [`apps/website/frontend/src/orbat_manager.rs`](../../apps/website/frontend/src/orbat_manager.rs) — Stitch near-fullscreen shell (`w-[min(1100px,95vw)]` + `max-w-6xl`); side tabs; template shell; live stats; search; squad/slot tree; inspector; OPEN ARSENAL → `open_attributes`
- Thin re-export from [`eden_chrome.rs`](../../apps/website/frontend/src/eden_chrome.rs)
- [`format_slot_line`](../../crates/map-engine-core/src/slot_line.rs) in map-engine-core (always-on)
- `editor_ops` mutators: `orbat_add_squad` / `orbat_add_slot` / `orbat_set_leader` / remove / rename / identity; `orbat_manager_snapshot`; `SquadRow.vehicle_ids`
- Windowed `__outlinerStats.orbat` for virtual-outliner v5
- Smoke o3: `Faction 1` → live `BLUFOR` + `Squad 1`
- Operator L8 kit-complement UI omitted (G4)

## Gates

### G3 — `format_slot_line`

```text
$ cargo test -p map-engine-core format_slot_line
test slot_line::tests::format_slot_line_primary_and_launcher ... ok
test slot_line::tests::format_slot_line_tag_med ... ok
test slot_line::tests::format_slot_line_is_leader ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out
```

### G5 / G6 — Add Squad / Add Role (core)

```text
$ cargo test -p map-engine-core --features doc orbat_add
test doc::place_orbat::tests::orbat_add_squad_increases_count_under_side ... ok
test doc::place_orbat::tests::orbat_add_role_increases_squad_slot_ids ... ok
```

### G1 / G2 / G7 / G8 — FE unit

```text
$ cargo test -p website-frontend
test orbat_manager::tests::g1_dialog_class_near_fullscreen ... ok
test orbat_manager::tests::g2_set_leader_symbol_in_module_source ... ok
test outliner::tests::orbat_manager_dialog_class_near_fullscreen ... ok
test outliner::tests::orbat_manager_empty_doc_empty_tree ... ok
test outliner::tests::orbat_side_tab_filters_by_faction_key ... ok
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### G2 — `set_leader` wiring

```text
$ rg -n 'set_leader|orbat_set_leader' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/editor_ops.rs
orbat_manager.rs:668: orbat_set_leader(...)
editor_ops.rs:1120: core.set_leader(&squad_id, &slot_id);
```

### G4 — no kit-complement UI strings

```text
$ rg -ni 'standardization|IFAK|Grenade Complement' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs && exit 1 || true
# (no matches)
```

### G9 — not max-w-xl-only

```text
$ rg -n 'max-w-xl' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs | head
orbat_manager.rs:26: ... comment (replaces T-177 max-w-xl)
orbat_manager.rs:884: assert!(!DIALOG_CLASS.contains("max-w-xl"));
# Dialog class uses w-[min(1100px,95vw)] + max-w-6xl
```

### `make ci-local-leptos` (fmt + clippy wasm32 + test + trunk)

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
cargo test -p website-frontend                 PASS (84)
trunk build --release                          PASS
  (ambient NO_COLOR=1 → trunk `--no-color` quirk;
   `env -u NO_COLOR -u FORCE_COLOR trunk build --release` ✅)
```

## Manual (operator)

| ID | Check |
|----|-------|
| M-G1 | Header tabs + toolbar + tree + inspector vs `screen.png` |
| M-G2 | Selected slot → inspector role/callsign/rank |
| M-G3 | Add Squad / Add Role / Make SL persist after close+reopen |
| M-G4 | No kit-complement block in inspector |

## Ready for

**T-180.8** — Template Apply + vehicles
