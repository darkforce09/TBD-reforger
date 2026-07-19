# T-180.6 verify log — Place/refile ↔ ORBAT + lines sync

**Date:** 2026-07-19  
**Slice:** [`t180_6_place_orbat_sync.md`](../../docs/specs/Mission_Creator_Architecture/t180_6_place_orbat_sync.md)  
**Tag:** `T-180.6`

## Shipped

- `editor_ops::refile_slot` → core `MissionDocCore::move_slot_to_squad` only + `after_local_edit` (orbat_nodes + squad_links)
- OrbatManager pointer-drag refile (slot → squad); miss-drop cancels armed refile
- SL badge from `squad.leaderSlotId` (`is_leader` on `OutlinerNode` / `FlatRow`)
- `squad_rows` reads `leaderSlotId`; no FE `slotIds` membership rewrite (F5)
- Core F4: `refile_merge_two_link_segments` (place×3 → merge → 2 segments)

## Gates

### F1 — two places ⇒ two squads

```text
$ cargo test -p map-engine-core --features doc two_places_two_squads_same_side
test doc::place_orbat::tests::two_places_two_squads_same_side ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 134 filtered out
```

### F2 — refile GC + bidirectional move

```text
$ cargo test -p map-engine-core --features doc empty_squad_garbage_collected
test doc::store::tests::empty_squad_garbage_collected ... ok

$ cargo test -p map-engine-core --features doc move_slot_bidirectional
test doc::store::tests::move_slot_bidirectional ... ok
```

### F3 / F-L6 — ORBAT tree + SL badge (FE)

```text
$ cargo test -p website-frontend
test outliner::tests::orbat_includes_two_squads_after_place_shaped_rows ... ok
test outliner::tests::orbat_sl_badge_from_leader_slot_id ... ok
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### F4 — merge size 3 ⇒ 2 segments + existing unit

```text
$ cargo test -p map-engine-core --features doc refile_merge_two_link_segments
test doc::place_orbat::tests::refile_merge_two_link_segments ... ok

$ cargo test -p map-engine-core squad_link_segment_count
test squad_links::tests::squad_link_segment_count ... ok
```

### F5 — no FE membership rewrite

```text
$ rg -n 'slotIds' apps/website/frontend/src/editor_ops.rs
708:                slot_ids: str_array(o.get("slotIds")),
941:/// (F-L2 — no FE `slotIds` splice), then the shared dirty tail (orbat_nodes + squad links).
```

Only `squad_rows` **read** + doc comment — no splice/insert of membership.

### `make ci-local-leptos`

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
  (pre-existing warnings only; no new errors)
cargo test -p website-frontend                 PASS (79)
trunk build --release                          PASS
  (make recipe / ambient NO_COLOR=1 → trunk `--no-color` quirk;
   `env -u NO_COLOR -u FORCE_COLOR trunk build --release` ✅)
```

## Locked pins (asserted)

| Pin | Value |
|-----|-------|
| Refile mutator | `move_slot_to_squad` only (no FE `slotIds` rewrite) |
| Post-edit tail | `after_local_edit` → `upload_squad_links` + `refresh_docks` |
| SL badge SoT | `squad.leaderSlotId` → `is_leader` (not `tag` / role text) |
| F4 merge | 3 places → 1 squad → **2** link segments |

## Manual

| ID | Status |
|----|--------|
| M-F1 | Operator: place two BLUFOR → two ORBAT squads → refile one into the other → one squad, SL badge, leader lines, empty GC |

## Ready for T-180.7

Yes — place/refile sync + SL badge live; next is Stitch ORBAT Manager UI.
