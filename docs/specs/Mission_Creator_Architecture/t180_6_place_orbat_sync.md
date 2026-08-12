# T-180.6 — Place / refile ↔ ORBAT live sync

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.2, T-180.4, T-180.5 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.  
**Verify log:** `.ai/artifacts/t180_6_verify_log.md`

## Problem

Operator loop: place from dock → ORBAT updates; refile into another squad → ORBAT + lines update; empty squad GC. Place already mints squads (.1/.5); need shared refile path + mirrors + line dirty.

## Locked

| ID | Decision |
|----|----------|
| F-L1 | Every `place_at` ends in `refresh_docks` / `orbat_nodes` including new squad |
| F-L2 | Refile = `move_slot_to_squad` + GC from .2 — **no** FE-only `slotIds` splice |
| F-L3 | After refile/place/set_leader → squad_link upload (.4 dirty path) |
| F-L4 | ORBAT Manager shell + Outliner mirrors same membership |
| F-L5 | No mock rows |
| F-L6 | SL badge in orbat tree from `leaderSlotId` |

## File map

| File | Change |
|------|--------|
| `apps/website/frontend/src/editor_ops.rs` | `refile_slot` → core `move_slot_to_squad`; shared after-edit |
| `apps/website/frontend/src/outliner.rs` | SL badge from `leaderSlotId` |
| `apps/website/frontend/src/eden_chrome.rs` | DnD refile onto squad rows (OrbatManager and/or dock mirrors) |
| `mission_history` / rebind | Already uploads squad links — ensure refile hits same path |

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **F1** | 2× place same side ⇒ 2 squads | `two_places_two_squads_same_side` (`--features doc`) |
| **F2** | Refile last member ⇒ source squad gone; dest has both | `refile_gc` or `empty_squad_garbage_collected` + move test |
| **F3** | `orbat_nodes` contains new squad after place (no modal) | FE/ops test |
| **F4** | Merge to size 3 with leader ⇒ 2 link segments | `squad_link_segment_count` inputs / integration |
| **F5** | `rg 'slotIds' apps/website/frontend/src/editor_ops.rs` shows no hand-rolled membership rewrite — only core calls | verify-log audit |

## Verify

```bash
cargo test -p map-engine-core --features doc two_places_two_squads_same_side
cargo test -p map-engine-core --features doc empty_squad_garbage_collected
cargo test -p map-engine-core --features doc move_slot_bidirectional
cargo test -p map-engine-core squad_link_segment_count
cargo test -p website-frontend --lib
cargo xtask mk ci-local-leptos
```

## Manual

M-F1: Place two BLUFOR → two squads in ORBAT. Refile one into the other → one squad, lines correct, empty squad gone.

## Claude Code prompt — T-180.6 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.6** — Place/refile ↔ ORBAT + lines sync.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_6_place_orbat_sync.md
  4. apps/website/frontend/src/editor_ops.rs (place_at, orbat_nodes refresh)
  5. crates/map-engine-core/src/doc/store.rs (move_slot_to_squad)
  6. mission_history squad_links upload path (T-180.4)

═══ PROBLEM ═══
  Place and refile must update orbat_nodes + leader lines via shared mutators + GC.
  No FE-only membership fork.

═══ SHIPPED ═══
  T-180.1–.5 @ aeb51209 / 83557768 / 19acc593 / 63e7ef00 / 1324799c

═══ LOCKED ═══
  - refile → move_slot_to_squad only
  - empty squad GC (.2)
  - lines rebuild on dirty (.4)
  - SL badge from leaderSlotId
  - No mock ORBAT

═══ DO ═══
  1. refile_slot / DnD → core move + after_local_edit
  2. Ensure orbat_nodes + squad_links refresh on place/refile
  3. SL badge in orbat tree
  4. Gates F1–F5 · .ai/artifacts/t180_6_verify_log.md · tag T-180.6

═══ DO NOT ═══
  Docs/registry · mock data · FE slotIds splice · Stitch full UI (.7)

═══ VERIFY ═══
  cargo test -p map-engine-core --features doc two_places_two_squads_same_side
  cargo test -p map-engine-core --features doc empty_squad_garbage_collected
  cargo test -p map-engine-core --features doc move_slot_bidirectional
  cargo test -p map-engine-core squad_link_segment_count
  cargo test -p website-frontend --lib
  cargo xtask mk ci-local-leptos

═══ MANUAL ═══
  M-F1: place two → refile merge → one squad + lines + GC

═══ RETURN ═══
  SHA + tag T-180.6 · verify log · Ready for T-180.7
```
