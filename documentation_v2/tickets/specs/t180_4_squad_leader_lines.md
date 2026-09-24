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
| D-L3 | Missing/invalid `leaderSlotId` or leader xy missing ⇒ 0 segments for that squad (no panic) |
| D-L4 | Stroke RGBA = `SIDE_*` from .3 (`side_rgba(side)`) |
| D-L5 | Rebuild on place/move/set_leader/refile/delete (doc_ver / position dirty) |
| D-L6 | Pure geometry in `map-engine-core`; upload via hairline LineList (~`engine.rs:4399`) |

## File map

| File | Change |
|------|--------|
| **NEW** `crates/map-engine-core/src/mission/squad_links.rs` (or `doc/` sibling) | `build_squad_link_segments` |
| `crates/map-engine-core/src/lib.rs` / `mission/mod.rs` | export |
| `crates/map-engine-render/src/engine.rs` | Upload lane for squad-link verts |
| FE / wasm | Feed squads+xy+side on dirty; do not invent geometry in Leptos |

## Pure function contract

```rust
/// LineList verts: [x0,y0,r,g,b,a, x1,y1,r,g,b,a, ...]  (2 verts/segment, 6 f32/vert)
pub fn build_squad_link_segments(
    squads: &[SquadLinkInput], // leader_slot_id, member_slot_ids, side: &str
    xy_by_slot: &HashMap<String, (f32, f32)>,
) -> Vec<f32>;
```

Colors: use `slots_gpu::side_rgba` / `SIDE_*_RGBA` — do not hardcode different values.

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **D1** | `slotIds.len()==5` + valid leader ⇒ **4** segments; verts.len()==**48** if 6×2×4 | `squad_link_segment_count` |
| **D2** | No segment with both endpoints non-leader | `no_peer_segments` |
| **D3** | size 1 ⇒ 0 | `solo_squad_zero_segments` |
| **D4** | OPFOR squad stroke == `SIDE_OPFOR_RGBA` as f32/255 | `squad_link_side_color` |
| **D5** | Two squads sizes 3+2 ⇒ 2+1 = 3 segments | `squad_link_multi_squad` |
| **D6** | Missing member xy ⇒ skip that segment only | `squad_link_skips_missing_xy` |

False-green: Outliner `guide_spans` only.

## Verify

```bash
cargo test -p map-engine-core squad_link_
cargo test -p map-engine-core --features doc squad_link_ 2>/dev/null || true
cargo test -p website-frontend --lib
cargo xtask mk ci-local-leptos
```

## Manual

M-D1: Two-man squad — one line. Add third — two lines from leader. Change SL — redraw. No peer line.

## Claude Code prompt — T-180.4 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.4** — Map squad leader→member lines.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_4_squad_leader_lines.md
  4. crates/map-engine-core/src/slots_gpu.rs (side_rgba / SIDE_*)
  5. crates/map-engine-core/src/doc/store.rs (leaderSlotId, slotIds)
  6. crates/map-engine-render/src/engine.rs (hairline LineList upload ~4399)

═══ PROBLEM ═══
  No map hierarchy lines. Need N−1 segments from leaderSlotId to each other member.
  Peer lines forbidden. Side-colored with .3 SIDE_* constants.

═══ SHIPPED ═══
  T-180.1 @ aeb51209 — leaderSlotId + place
  T-180.2 @ 83557768 — set_leader / move / GC
  T-180.3 @ 19acc593 — SIDE_* + side_rgba + SlotSoa.side_keys

═══ LANGUAGE GATE ═══
  Geometry in map-engine-core (squad_links). Upload in map-engine-render.
  Leptos: thin dirty→upload only. STOP IF inventing segment math in FE.

═══ LOCKED ═══
  - N members total ⇒ N−1 segments; solo ⇒ 0
  - No peer segments
  - Colors via side_rgba / SIDE_* (not new hex)
  - Dock guide_spans ≠ this slice
  - Not T-080 module sync

═══ DO ═══
  1. build_squad_link_segments + tests D1–D6
  2. Engine lane + wire on place/move/set_leader/refile
  3. .ai/artifacts/t180_4_verify_log.md · tag T-180.4

═══ DO NOT ═══
  Docs/registry · peer mesh · count dock guides as done · defer lines

═══ VERIFY ═══
  cargo test -p map-engine-core squad_link_
  cargo test -p website-frontend --lib
  cargo xtask mk ci-local-leptos

═══ MANUAL ═══
  M-D1: leader→members only; SL change redraws

═══ RETURN ═══
  SHA + tag T-180.4 · verify log · Ready for T-180.5
```
