# T-180.7 — Stitch ORBAT Manager UI (visual + live graph)

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.2, T-180.6 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Verify log:** [`.ai/artifacts/t180_7_verify_log.md`](../../../.ai/artifacts/t180_7_verify_log.md) (create on ship)

## Stitch mock-up (operator) — REQUIRED READ

| Asset | Path |
|-------|------|
| Screenshot | [`.ai/artifacts/t180_stitch_orbat_modal/screen.png`](../../../.ai/artifacts/t180_stitch_orbat_modal/screen.png) |
| Static HTML | [`.ai/artifacts/t180_stitch_orbat_modal/code.html`](../../../.ai/artifacts/t180_stitch_orbat_modal/code.html) |
| Tokens | [`.ai/artifacts/t180_stitch_orbat_modal/DESIGN.md`](../../../.ai/artifacts/t180_stitch_orbat_modal/DESIGN.md) |

**These are visual authority for layout/chrome.** Data must be live mission doc — Stitch sample strings (L85A3, US 1980s) are examples, not hardcoded fixtures.

---

## Stitch region inventory → implement / defer

| Stitch region (from `screen.png` / `code.html`) | In T-180.7? | Notes |
|--------------------------------------------------|-------------|-------|
| Title **ORBAT MANAGER** + close | YES | Near-fullscreen dialog |
| Side tabs **BLUFOR / OPFOR / INDFOR** | YES | Filter tree to side; match L1 |
| Template dropdown + **APPLY TEMPLATE** | SHELL in .7 / full in **.8** | .7: disabled or list library names; Apply wiring completes in .8 |
| Stats Total Entities / Vehicles | YES | Live counts from doc for active side (or mission-wide — prefer active side) |
| Search entities | YES | Filter squad/slot labels |
| Squad header (name, expand, vehicle badge e.g. M113) | YES | Vehicle badge needs .8 data; show if `vehicleIds` non-empty |
| Slot rows `N: Role (weapons) \| TAG` + drag handle | YES | `format_slot_line`; SL icon when `leaderSlotId` |
| Hover edit / remove slot | YES | Wire to mutators |
| Squad actions: Add Slot, Add Vehicle, Remove Squad | Add Slot YES; Add Vehicle shell→**.8**; Remove YES |
| **+ Add Role** under squad | YES | = add slot (role picker or default Rifleman) |
| **+ ADD SQUAD / GROUP** | YES | `add_squad` under active side |
| Slot Inspector: Entity type, Role, Callsign, Rank | YES | callsign/rank from T-180.1 fields |
| Loadout weight + **OPEN ARSENAL** | Button YES; weight optional; Arsenal navigate can land in .9 if attrs seam hard — prefer wire in .7 |
| **Standardization** (IFAK / grenades) | **NO — omit** | Operator deferred L8 |
| Mock tree content | **NO** | Live `build_orbat` / doc only |

---

## Problem

[`OrbatManagerDialog`](../../../apps/website/frontend/src/eden_chrome.rs) (~1200) is `max-w-xl` browse/select + T-071.1 stub footer. Not Stitch. No Make SL, no inspector, no Add Squad/Role on live mutators.

---

## File map

| File | Change |
|------|--------|
| `apps/website/frontend/src/eden_chrome.rs` | Replace dialog body **or** thin wrapper |
| **NEW preferred** `apps/website/frontend/src/orbat_manager.rs` | Full Stitch layout module; keep `eden_chrome` export thin |
| `crates/map-engine-core/src/mission/slot_line.rs` (or FE pure) | `format_slot_line(index, role, summary/primary/launcher, tag, is_leader)` — **prefer Rust** for Class-R tests |
| `apps/website/frontend/src/outliner.rs` | Reuse tree row patterns / `build_orbat` if helpful |
| `apps/website/frontend/src/mission_editor.rs` | Mount; pass doc handles / active side |

Layout targets (from Stitch, not pixel-perfect compulsory but structure yes):

- Dialog ≈ `w-[min(1100px,95vw)] h-[min(800px,90vh)]` or `max-w-6xl` + tall — **not** `max-w-xl` alone  
- Header tabs centered; toolbar template+stats; main = tree \| inspector (~1/3)

---

## Slot line algorithm

```text
1-based index from sorted slot.index within squad
weapons =
  if loadout.summary has " · " style → map to "(Primary + Launcher)" for UI
  prefer: primary display + optional " + " + launcher display inside parens
  if only primary → "(Primary)"
  if none → omit parens
tag part = if tag nonempty → " | {tag}"
SL: leading military_tech icon and/or append " | SL" — do NOT overwrite tag field
Example: 1: Squad Leader (L85A3 + GL)
Example: 2: Medic (L85A3) | MED
```

---

## Class-R gates

| ID | Assert | How |
|----|--------|-----|
| **G1** | OrbatManager dialog classes include near-fullscreen width (≥ `max-w-4xl` or `w-[min(`) — not only `max-w-xl` | `rg` + unit/string assert |
| **G2** | Make SL / toggle SL calls `set_leader` (core) | wiring test or `rg set_leader` from orbat_manager |
| **G3** | `format_slot_line` tests: primary+launcher; tag MED; is_leader | `format_slot_line_*` in core or FE tests |
| **G4** | Standardization strings absent | `rg -i 'standardization\|IFAK\|Grenade Complement' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs` → 0 |
| **G5** | Add Squad increases `squadsById` count under active side | FE/ops test |
| **G6** | Add Role increases slotIds on selected/new squad | FE/ops test |
| **G7** | No hardcoded "L85A3" / "US 1980s" as Sole data source when doc empty | empty doc → empty tree / empty-state, not Stitch sample |
| **G8** | Side tab OPFOR shows only squads under faction.`key=="OPFOR"` (not name substring) | filter test — requires `FactionRow.key` from .1 |
| **G9** | `rg -n 'max-w-xl' orbat_manager.rs eden OrbatManager` — if `max-w-xl` remains as sole width constraint → FAIL; near-fullscreen class required | shell + assert |

---

## Verify

```bash
cargo test -p map-engine-core format_slot_line 2>/dev/null || cargo test format_slot_line
make ci-local-leptos
rg -n 'max-w-xl' apps/website/frontend/src/eden_chrome.rs apps/website/frontend/src/orbat_manager.rs | head
# G4:
rg -ni 'standardization|IFAK|Grenade Complement' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs && exit 1 || true
```

## Manual (eye-pass vs Stitch)

| ID | Check against `screen.png` |
|----|----------------------------|
| M-G1 | Header tabs + toolbar + tree + inspector present |
| M-G2 | Selected slot highlights; inspector shows role/callsign/rank |
| M-G3 | Add Squad / Add Role / Make SL persist after close+reopen |
| M-G4 | No Standardization block |

---

## Claude Code prompt — T-180.7 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.7** — Stitch ORBAT Manager UI on live data.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_7_orbat_manager_ui.md
  4. docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md (L1–L10; L8 Standardization omit)
  5. .ai/artifacts/t180_stitch_orbat_modal/screen.png
  6. .ai/artifacts/t180_stitch_orbat_modal/code.html
  7. .ai/artifacts/t180_stitch_orbat_modal/DESIGN.md
  8. apps/website/frontend/src/eden_chrome.rs OrbatManagerDialog
  9. apps/website/frontend/src/editor_ops.rs (refile_slot, after_local_edit — T-180.6)
  10. crates/map-engine-core mutators (set_leader, add_squad, move_slot_to_squad, …)

═══ PROBLEM ═══
  Narrow browse shell (max-w-xl). Rebuild to Stitch layout on LIVE graph.
  No mock ORBAT. Omit Standardization (operator L8). Template Apply full wire → .8;
  Open Arsenal full wire → .9 if attrs seam hard (button OK in .7).

═══ SHIPPED ═══
  T-180.1–.6 @ aeb51209 / 83557768 / 19acc593 / 63e7ef00 / 1324799c / 056c9a1a

═══ LOCKED ═══
  - Visual authority = stitch artifacts (structure; not pixel-perfect compulsory)
  - Live data only (G7) — empty doc ⇒ empty-state, not Stitch sample strings
  - SL via leaderSlotId / set_leader (G2); do not overwrite MED/ENG tag
  - No Standardization / IFAK / Grenade Complement (G4)
  - Near-fullscreen width — not max-w-xl-only (G1/G9)
  - Side tabs filter by FactionRow.key (G8)
  - Add Slot / Add Squad / Remove / rename / search / inspector callsign/rank/role
  - Template dropdown shell OK; Apply completes in .8
  - Add Vehicle shell→.8; vehicle badge if vehicleIds non-empty

═══ DO ═══
  1. Prefer new apps/website/frontend/src/orbat_manager.rs from code.html; thin eden_chrome export
  2. format_slot_line (+ tests G3) — prefer Rust in map-engine-core
  3. Wire Add Squad/Role, Make SL, rename, remove, search, side tabs, refile (reuse .6)
  4. Inspector; OPEN ARSENAL button (navigate .9 OK if hard)
  5. Gates G1–G9 · .ai/artifacts/t180_7_verify_log.md · tag T-180.7

═══ DO NOT ═══
  Docs/registry · mock Stitch sample as SoT · Standardization · FE membership fork
  · skip gates · implement full T-153 Apply (.8) / compile loadout (.9)

═══ VERIFY ═══
  cargo test -p map-engine-core format_slot_line
  cargo test -p website-frontend --lib
  make ci-local-leptos
  rg -n 'max-w-xl' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs | head
  rg -ni 'standardization|IFAK|Grenade Complement' apps/website/frontend/src/orbat_manager.rs apps/website/frontend/src/eden_chrome.rs && exit 1 || true

═══ MANUAL ═══
  M-G1..M-G4 vs .ai/artifacts/t180_stitch_orbat_modal/screen.png

═══ RETURN ═══
  SHA + tag T-180.7 · verify log · Ready for T-180.8
```
