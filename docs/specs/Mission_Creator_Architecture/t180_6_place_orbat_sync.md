# T-180.6 — Place / refile ↔ ORBAT live sync

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.2, T-180.4, T-180.5 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Verify log:** `.ai/artifacts/t180_6_verify_log.md`

## Problem

Operator loop: place from dock → appears in ORBAT; refile into another squad → ORBAT + lines update; empty squad deleted. Today place mints squads (.1) but refile UI / dock mirrors / line dirty may not share one path.

## Locked

| ID | Decision |
|----|----------|
| F-L1 | Every `place_at` ends in `refresh_docks` / `orbat_nodes` including new squad |
| F-L2 | Refile uses `move_slot_to_squad` + GC from .2 — no third membership implementation |
| F-L3 | After refile/place/set_leader, squad_link segments rebuild (.4) |
| F-L4 | ORBAT Manager tree (even pre-.7 shell) and Outliner mirrors show same membership |
| F-L5 | No mock rows |

## File map

| File | Change |
|------|--------|
| `apps/website/frontend/src/editor_ops.rs` | `refile_slot`; ensure place/refile/set_leader all call shared after-edit |
| `apps/website/frontend/src/outliner.rs` | SL badge from `leaderSlotId` in orbat tree build |
| `apps/website/frontend/src/eden_chrome.rs` | DnD refile onto squad rows if present |
| Line dirty | Call squad_links rebuild |

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **F1** | 2× place same side ⇒ 2 squads | `two_places_two_squads` (.1 may exist) |
| **F2** | Refile last member ⇒ source squad gone; dest has both | `refile_gc` |
| **F3** | orbat_nodes contains new squad after place without opening modal | FE/ops test |
| **F4** | After merge to size 3 with leader ⇒ segment count 2 | uses `squad_link_segment_count` inputs |
| **F5** | Refile does not call a second hand-rolled slotIds splice in FE | `rg` audit / code review gate in verify log |

## Verify

```bash
cargo test -p map-engine-core refile_gc
cargo test -p map-engine-core two_places_two_squads
cargo test -p map-engine-core squad_link_segment_count
make ci-local-leptos
```

## Manual

M-F1: Place two BLUFOR units → two squads in ORBAT tree. Drag one into the other → one squad, one line (or two if 3 people), empty squad gone.

## Claude Code prompt — T-180.6 (copy-paste)

```
Read CLAUDE.md first.
Implement **T-180.6** — Place/refile ↔ ORBAT + lines sync.

═══ READ ═══
  t180_6_place_orbat_sync.md · hub · handoff · editor_ops.rs · T-180.2 mutators · T-180.4 links

═══ PROBLEM ═══
  Place and refile must update orbat_nodes + lines via shared mutators + GC.

═══ DO ═══
  1. Wire refile → move_slot_to_squad
  2. Ensure dock refresh + line dirty
  3. Gates F1–F5 · tag T-180.6

═══ DO NOT ═══
  Docs · mock ORBAT · FE-only membership fork

═══ VERIFY ═══
  cargo test -p map-engine-core refile_gc two_places_two_squads
  make ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.6 · Ready for T-180.7
```
