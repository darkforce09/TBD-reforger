# T-180.2 — Graph mutators + empty-squad GC

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.  
**Verify log:** `.ai/artifacts/t180_2_verify_log.md`

## Problem

UI surfaces (ORBAT Manager, refile, Make SL, vehicles) must share one mutator layer. Today [`store.rs`](../../../crates/map-engine-core/src/doc/store.rs) has `add_faction` / `add_squad` / `add_slot` / `remove_slot` / `update_slot_loadout` — **no** `set_leader`, **no** `move_slot_to_squad`, **no** empty-squad delete, **no** `vehicleIds` attach.

Without this, T-180.4 lines and T-180.6/7 sync invent parallel membership rules (forbidden).

## Locked

| ID | Decision |
|----|----------|
| B-L1 | `set_leader(squad_id, slot_id)` requires `slot_id ∈ squad.slotIds`; clears prior leader |
| B-L2 | Moving last slot out of squad **deletes** squad (and detaches vehicles or deletes orphan vehicles — pick: **delete vehicles attached only to that squad**) |
| B-L3 | `move_slot_to_squad` updates `slot.squadId`, both `slotIds` arrays, rewrites `index` densely 0..n-1 on both squads |
| B-L4 | If moved slot was leader of source: new leader = first remaining slot or squad deleted |
| B-L5 | If dest had no leader and gains first slot → that slot becomes leader |
| B-L6 | `vehicleIds: string[]` on squad; `attach_vehicle` / `detach_vehicle` |
| B-L7 | All mutators undoable via existing txn / LOCAL_ORIGIN patterns |
| B-L8 | **`add_vehicle` ABSENT today** ([`t180_class_r_pins.md`](t180_class_r_pins.md)) — add `add_vehicle` mutator writing `vehiclesById` row `{id, …}` so .8 is not blocked; can be minimal (id + resourceName + optional position) |
| B-L9 | `move_slot_to_layer` ([`store.rs:853`](../../../crates/map-engine-core/src/doc/store.rs)) is **not** squad refile — do not reuse it for ORBAT |

## File map

| File | Change |
|------|--------|
| `crates/map-engine-core/src/doc/store.rs` | All mutators above + helpers |
| `crates/map-engine-core/src/doc/` tests | B1–B7 |
| Optional thin FE | `editor_ops.rs` wrappers calling core — no logic fork |

## API sketch (Rust)

```rust
fn set_leader(&self, squad_id: &str, slot_id: &str) -> Result<(), OrbatErr>;
fn rename_squad(&self, squad_id: &str, name: &str);
fn reorder_squads(&self, faction_id: &str, squad_ids: &[String]); // writes faction.squadIds
fn move_slot_to_squad(&self, slot_id: &str, dest_squad_id: &str);
fn remove_squad(&self, squad_id: &str); // deletes slots or refuses if nonempty — prefer cascade slots
fn attach_vehicle(&self, squad_id: &str, vehicle_id: &str);
fn detach_vehicle(&self, squad_id: &str, vehicle_id: &str);
fn ensure_leader_invariant(&self, squad_id: &str); // internal after mutations
```

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **B1** | set_leader(B) after leader A ⇒ leaderSlotId=B only | `set_leader_exclusive` |
| **B2** | move last slot away ⇒ squad key absent from squads map | `empty_squad_garbage_collected` |
| **B3** | move_slot: source slotIds without id; dest with id; slot.squadId=dest | `move_slot_bidirectional` |
| **B4** | After every mutator fixture: leader ∈ slotIds or squad gone | `leader_invariant_holds` |
| **B5** | move leader away with members left ⇒ remaining[0] is leader | `move_leader_promotes_next` |
| **B6** | `add_vehicle` + attach then detach `vehicleIds` | `attach_vehicle_roundtrip` |
| **B7** | Dense index rewrite 0..n-1 after reorder/move | `slot_indices_dense_after_move` |
| **B8** | `rg -n 'fn add_vehicle' crates/map-engine-core` exits 0 | shell |

## Verify

```bash
# MissionDocCore tests require --features doc (T-180.1 measured)
cargo test -p map-engine-core --features doc set_leader_exclusive
cargo test -p map-engine-core --features doc empty_squad_garbage_collected
cargo test -p map-engine-core --features doc move_slot_bidirectional
cargo test -p map-engine-core --features doc leader_invariant_holds
cargo test -p map-engine-core --features doc move_leader_promotes_next
cargo test -p map-engine-core --features doc attach_vehicle_roundtrip
cargo test -p map-engine-core --features doc slot_indices_dense_after_move
if ! rg -n 'fn add_vehicle' crates/map-engine-core; then echo 'B8 FAIL'; exit 1; fi
cargo test -p website-frontend --lib
cargo xtask mk ci-local-leptos
```

## Claude Code prompt — T-180.2 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.2** — Graph mutators + empty-squad GC.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_2_graph_mutators.md
  4. docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md
  5. crates/map-engine-core/src/doc/store.rs
  6. crates/map-engine-core/src/doc/place_orbat.rs (T-180.1 — preserve invariants)

═══ PROBLEM ═══
  Need move_slot_to_squad, rename/reorder, empty-squad GC, vehicleIds, add_vehicle.
  set_leader exists from .1 — extend exclusivity tests. No parallel membership in Leptos.

═══ SHIPPED ═══
  T-180.1 @ aeb51209 — place_character_under_side, set_leader, callsign/rank, FactionRow.key

═══ LOCKED ═══
  - Exclusive leader; empty squad deleted; dense index 0..n-1
  - add_vehicle required (ABSENT pre-.2) — B8
  - Do not reuse move_slot_to_layer as squad refile
  - Core tests: cargo test -p map-engine-core --features doc …

═══ DO ═══
  1. Mutators in store.rs (+ add_vehicle)
  2. Tests B1–B8
  3. .ai/artifacts/t180_2_verify_log.md · tag T-180.2

═══ DO NOT ═══
  Docs/registry · Stitch UI · map lines · defer GC · skip add_vehicle

═══ VERIFY ═══
  cargo test -p map-engine-core --features doc set_leader_exclusive
  cargo test -p map-engine-core --features doc empty_squad_garbage_collected
  cargo test -p map-engine-core --features doc move_slot_bidirectional
  cargo test -p map-engine-core --features doc leader_invariant_holds
  cargo test -p map-engine-core --features doc move_leader_promotes_next
  cargo test -p map-engine-core --features doc attach_vehicle_roundtrip
  cargo test -p map-engine-core --features doc slot_indices_dense_after_move
  if ! rg -n 'fn add_vehicle' crates/map-engine-core; then exit 1; fi
  cargo test -p website-frontend --lib
  cargo xtask mk ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.2 · .ai/artifacts/t180_2_verify_log.md · Ready for T-180.3
```

