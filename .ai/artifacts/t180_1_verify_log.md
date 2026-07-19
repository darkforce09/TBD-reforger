# T-180.1 verify log — Foundation schema + place→new squad

**Date:** 2026-07-19  
**Slice:** [`t180_1_foundation_schema.md`](../../docs/specs/Mission_Creator_Architecture/t180_1_foundation_schema.md)  
**Tag:** `T-180.1`

## Gates

### A1 — `place_character_under_side_opfor`

```text
$ cargo test -p map-engine-core --features doc place_character_under_side_opfor
test doc::place_orbat::tests::place_character_under_side_opfor ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out
```

(`--features doc` required — `MissionDocCore` is feature-gated; bare filter without features matches 0 tests.)

### A2 — `slot_callsign_rank_roundtrip`

```text
$ cargo test -p map-engine-core --features doc slot_callsign_rank_roundtrip
test doc::place_orbat::tests::slot_callsign_rank_roundtrip ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out
```

### A3 — zero `ensure_default_squad` in `editor_ops.rs`

```text
$ if rg -n 'ensure_default_squad' apps/website/frontend/src/editor_ops.rs; then echo 'A3 FAIL'; exit 1; else echo 'A3 PASS (zero matches)'; fi
A3 PASS (zero matches)
```

### A4 — `two_places_two_squads_same_side`

```text
$ cargo test -p map-engine-core --features doc two_places_two_squads_same_side
test doc::place_orbat::tests::two_places_two_squads_same_side ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out
```

### A5 — `place_rejects_invalid_side`

```text
$ cargo test -p map-engine-core --features doc place_rejects_invalid_side
test doc::place_orbat::tests::place_rejects_invalid_side ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out
```

### A6 — faction `key` plumbed

Covered by A1 (`faction-OPFOR.key == "OPFOR"`) + FE `FactionRow.key` / `faction_rows` read path.

### A7 — `place_at` has no `DEFAULT_SQUAD_ID`

```text
$ rg -n 'DEFAULT_SQUAD_ID' apps/website/frontend/src/editor_ops.rs
# (no matches)
$ rg -n 'place_at' apps/website/frontend/src/editor_ops.rs
9://! **Placement (T-180.1):** each `place_at` calls
815:/// Palette leaf `pointerdown` → arm a place. Consumed by [`place_at`] on a canvas release, or
906:pub fn place_at(x: f64, y: f64) -> bool {
```

`place_at` body calls `place_character_under_side` with `active_side` only.

### FE + CI

```text
$ cargo test -p website-frontend
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ env -u NO_COLOR -u FORCE_COLOR make ci-local-leptos
# fmt check + clippy wasm32 + cargo test + trunk build --release → ✅ success
```

Note: sandbox/CI shells that export `NO_COLOR=1` break `trunk 0.21.14` (`invalid value '1' for '--no-color'`). Unset `NO_COLOR`/`FORCE_COLOR` for the trunk step.

## Shipped

| Item | Location |
|------|----------|
| `set_leader` + `update_slot_identity` | `crates/map-engine-core/src/doc/store.rs` |
| `place_character_under_side` + A1–A5 | `crates/map-engine-core/src/doc/place_orbat.rs` |
| `active_side` + `place_at` wire; dump path removed | `apps/website/frontend/src/editor_ops.rs` |
| `FactionRow.key` | `outliner.rs` + `faction_rows` |
| OpsCtx / mission_editor signal | `mission_editor.rs` |

## Ready for

**T-180.2** — graph mutators + empty-squad GC.
