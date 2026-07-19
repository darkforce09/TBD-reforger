# T-180.2 verify log — Graph mutators + empty-squad GC

**Date:** 2026-07-19  
**Slice:** [`t180_2_graph_mutators.md`](../../docs/specs/Mission_Creator_Architecture/t180_2_graph_mutators.md)  
**Tag:** `T-180.2`

## Gates

### B1 — `set_leader_exclusive`

```text
$ cargo test -p map-engine-core --features doc set_leader_exclusive
test doc::store::tests::set_leader_exclusive ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B2 — `empty_squad_garbage_collected`

```text
$ cargo test -p map-engine-core --features doc empty_squad_garbage_collected
test doc::store::tests::empty_squad_garbage_collected ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B3 — `move_slot_bidirectional`

```text
$ cargo test -p map-engine-core --features doc move_slot_bidirectional
test doc::store::tests::move_slot_bidirectional ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B4 — `leader_invariant_holds`

```text
$ cargo test -p map-engine-core --features doc leader_invariant_holds
test doc::store::tests::leader_invariant_holds ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B5 — `move_leader_promotes_next`

```text
$ cargo test -p map-engine-core --features doc move_leader_promotes_next
test doc::store::tests::move_leader_promotes_next ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B6 — `attach_vehicle_roundtrip`

```text
$ cargo test -p map-engine-core --features doc attach_vehicle_roundtrip
test doc::store::tests::attach_vehicle_roundtrip ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B7 — `slot_indices_dense_after_move`

```text
$ cargo test -p map-engine-core --features doc slot_indices_dense_after_move
test doc::store::tests::slot_indices_dense_after_move ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### B8 — `fn add_vehicle` present

```text
$ rg -n 'fn add_vehicle' crates/map-engine-core
crates/map-engine-core/src/doc/store.rs:484:    pub fn add_vehicle(
B8 PASS
```

### Regression — place_orbat (T-180.1)

```text
$ cargo test -p map-engine-core --features doc place_character_under_side
test doc::place_orbat::tests::place_character_under_side_opfor ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out
```

### FE + CI

```text
$ cargo test -p website-frontend
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ env -u NO_COLOR -u FORCE_COLOR make ci-local-leptos
# fmt check + clippy wasm32 + cargo test + trunk build --release → ✅ success
```

Note: `cargo test -p website-frontend --lib` fails with `no library targets` (bin/wasm package); use bare `cargo test -p website-frontend` as in T-180.1.

## Shipped

| Item | Location |
|------|----------|
| Harden `set_leader` (member-only) | `crates/map-engine-core/src/doc/store.rs` |
| `rename_squad` / `reorder_squads` | same |
| `move_slot_to_squad` + empty-squad GC + dense index | same |
| `remove_squad` (cascade slots + vehicles) | same |
| `add_vehicle` / `attach_vehicle` / `detach_vehicle` | same |
| `vehicleIds: []` on `add_squad`; vehicles undo scope | same |
| B1–B7 unit tests | `store.rs` `#[cfg(test)]` |

## Ready for

**T-180.3** — map side tint.
