# T-180.4 verify log — Map squad leader→member lines

**Date:** 2026-07-19  
**Slice:** [`t180_4_squad_leader_lines.md`](../../docs/specs/Mission_Creator_Architecture/t180_4_squad_leader_lines.md)  
**Tag:** `T-180.4`

## Shipped

- Always-on `crates/map-engine-core/src/squad_links.rs`: `SquadLinkInput` + `build_squad_link_segments` (N−1 leader→member, `side_rgba` → f32/255, no peers)
- `MissionDocCore::squad_link_inputs()` — collect leader / members / side (no geometry)
- Engine `LaneRole::SquadLinks` · `lane_role_from_u32(9)` · draw order Grid < SquadLinks < Slots
- Leptos `mission_history`: `after_doc_change` + `rebind_engine_from_doc` → `upload_hairline_segments(9, …)`

## Gates

### D1–D6 — `squad_link_`

```text
$ cargo test -p map-engine-core squad_link_
test squad_links::tests::squad_link_multi_squad ... ok
test squad_links::tests::squad_link_no_peer_segments ... ok
test squad_links::tests::squad_link_segment_count ... ok
test squad_links::tests::squad_link_skips_missing_xy ... ok
test squad_links::tests::squad_link_side_color ... ok
test squad_links::tests::squad_link_solo_zero_segments ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 104 filtered out
```

### Draw-order pin

```text
$ cargo test -p map-engine-render squad_links_sit
test draw_order::lane_order_pins::squad_links_sit_between_grid_and_slots ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out
```

### Frontend unit tests

```text
$ cargo test -p website-frontend
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### `make ci-local-leptos` (clippy + test + trunk)

```text
cargo fmt -p website-frontend --check          PASS
cargo clippy -p website-frontend --target wasm32-unknown-unknown  PASS
  (pre-existing warnings only; no new errors)
cargo test -p website-frontend                 PASS (74)
trunk build --release                          PASS
  (make recipe / ambient NO_COLOR=1 → trunk `--no-color` quirk;
   `env -u NO_COLOR -u FORCE_COLOR trunk build --release` ✅)
```

## Locked pins (asserted)

| Pin | Value |
|-----|-------|
| ROLE_SQUAD_LINKS | `9` |
| D1 verts | 4 segs × 12 f32 = **48** |
| OPFOR stroke | `SIDE_OPFOR_RGBA` as f32/255 |
| Draw order | Grid < SquadLinks < Slots |

## Manual

| ID | Status |
|----|--------|
| M-D1 | Operator: two-man squad → one line; add third → two from leader; change SL → redraw; no peer line |

## Ready for T-180.5

Yes — right dock Eden chips + Objects stub next.
