# T-180.4 — Map squad leader lines (absorbs T-147)

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.2, T-180.3 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Verify log:** `.ai/artifacts/t180_4_verify_log.md`

## Problem

No entity hierarchy lines on the map. Operator: lines **only** Squad Leader → each other member; never peer↔peer; side-colored.

**Not T-080** (Eden sync to modules/triggers). Dock `guide_spans` UI stems do **not** count.

## Locked

| ID | Decision |
|----|----------|
| D-L1 | Segment endpoints = (leader xy, member xy) for each member ≠ leader in same squad |
| D-L2 | Squad size N including leader ⇒ **N−1** segments; N=1 ⇒ 0 |
| D-L3 | No leaderSlotId or leader missing from map ⇒ 0 segments for that squad (or treat sole member as leader if invariant holds from .2) |
| D-L4 | Stroke RGBA = side tint from .3 |
| D-L5 | Update on place/move/set_leader/refile/delete (dirty with doc_ver / position) |
| D-L6 | Pure geometry in `map-engine-core`; upload via existing hairline LineList path in render (~`engine.rs:4399`) |

## File map

| File | Change |
|------|--------|
| **NEW** `crates/map-engine-core/src/mission/squad_links.rs` | `build_squad_link_segments(...)` → verts/segments |
| `crates/map-engine-core/src/lib.rs` | `mod mission::squad_links` |
| `crates/map-engine-render/src/engine.rs` | Dedicated lane or reuse hairline upload |
| FE / wasm bridge | Push segments when slots/leaders/xy change |

## Pure function contract

```rust
/// Returns LineList verts: [x0,y0,r,g,b,a, x1,y1,r,g,b,a, ...]  (2 verts/segment)
pub fn build_squad_link_segments(
    squads: &[SquadLinkInput], // id, leader_slot_id, member_slot_ids, side
    xy_by_slot: &HashMap<String, (f32, f32)>,
) -> Vec<f32>;
```

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **D1** | Squad with `slotIds.len()==5` and valid leader ⇒ **exactly 4** segments (= N−1). If using 6 floats/vert × 2 verts/seg ⇒ **48** f32s | `squad_link_segment_count` — `assert_eq!(segs, 4)` and/or `assert_eq!(verts.len(), 48)` |
| **D2** | No segment both endpoints non-leader | `no_peer_segments` |
| **D3** | size 1 ⇒ 0 | `solo_squad_zero_segments` |
| **D4** | Side color matches SIDE_* constants | `squad_link_side_color` |
| **D5** | Two squads ⇒ sum of per-squad (N_i−1) | `squad_link_multi_squad` |
| **D6** | Missing xy for member ⇒ skip that segment only (no panic) | `squad_link_skips_missing_xy` |

False-green: drawing Outliner guides only.

## Verify

```bash
cargo test -p map-engine-core squad_link_
make ci-local-leptos
```

## Manual

M-D1: Two-man squad — one line leader→member. Add third — two lines from leader. Set other SL — lines redraw. No line between the two non-leaders.

## Claude Code prompt — T-180.4 (copy-paste)

```
Read CLAUDE.md first.
Implement **T-180.4** — leader→member map lines.

═══ READ ═══
  t180_4_squad_leader_lines.md · hub · handoff
  slots_gpu.rs (side colors) · engine.rs LineList ~4399

═══ PROBLEM ═══
  Need N-1 segments per squad from leaderSlotId. No peer lines. Side-colored.

═══ LANGUAGE GATE ═══
  Geometry in squad_links.rs (Rust). Render upload in map-engine-render.

═══ DO ═══
  1. Pure builder + D1–D6 tests
  2. Engine lane + wire dirty
  3. verify log · tag T-180.4

═══ DO NOT ═══
  Docs · T-080 module sync · peer mesh · count dock guides as done

═══ VERIFY ═══
  cargo test -p map-engine-core squad_link_
  make ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.4 · Ready for T-180.5
```
