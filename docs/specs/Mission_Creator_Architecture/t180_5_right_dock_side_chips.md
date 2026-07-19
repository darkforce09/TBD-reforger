# T-180.5 — Right dock Eden side chips (absorbs T-074)

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.  
**Visual ref:** [`.ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png`](../../../.ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png)  
**Verify log:** `.ai/artifacts/t180_5_verify_log.md`

## Problem (measured)

[`DockRight`](../../../apps/website/frontend/src/eden_chrome.rs): tabs Factions / Vehicles / Markers + search. **No** BLUFOR/OPFOR/INDFOR/Objects chips. `active_side` from T-180.1 cannot be set from UI.

## Locked

| ID | Decision |
|----|----------|
| E-L1 | Chip row: **blue BLUFOR, red OPFOR, green INDFOR, yellow Objects** — **no F1–F6**, **no CIV** |
| E-L2 | Side chip sets `OpsCtx.active_side` to exact `"BLUFOR"\|"OPFOR"\|"INDFOR"` |
| E-L2b | Catalog may stay unfiltered if registry lacks side tags — **document in verify log**; **E4 place side is the hard gate** |
| E-L3 | Objects mode: empty / “Objects coming soon…” — place no-op; no panic |
| E-L4 | Chips above search; keep Factions/Vehicles/Markers tabs as today |
| E-L5 | Search still filters visible catalog when in units mode |

## File map

| File | Change |
|------|--------|
| `apps/website/frontend/src/eden_chrome.rs` | Chip UI + catalog mode |
| `apps/website/frontend/src/asset_catalog.rs` | Objects empty-state helper if needed |
| `apps/website/frontend/src/editor_ops.rs` / `mission_editor.rs` | Chips write `active_side` (already exists from .1) |

## Class-R gates

| ID | Assert | How |
|----|--------|-----|
| **E1** | Side chip UI present; no F1–F6 mode row in DockRight | `rg` + FE test / snapshot |
| **E2** | Setting chip OPFOR ⇒ `active_side == "OPFOR"` | FE/ops test |
| **E3** | Objects mode shows coming-soon / empty string | FE test |
| **E4** | With `active_side=OPFOR`, `place_character_under_side` / place path → `faction-OPFOR` | core `--features doc` or ops test |
| **E5** | No CIV chip | assert chip list len/labels |

## Verify

```bash
cargo test -p website-frontend --lib
cargo test -p map-engine-core --features doc place_character_under_side_opfor
# E1: no F-key palette row (allow icon fonts unrelated):
rg -n 'F1|F2|F3|F4|F5|F6' apps/website/frontend/src/eden_chrome.rs | head
make ci-local-leptos
```

## Manual

M-E1: Eye-pass vs `eden_side_chips_ref.png` (color chips + search; ignore F1–F6).  
M-E2: OPFOR chip → place → red ring (.3) + squad links (.4) if multi.

## Claude Code prompt — T-180.5 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.5** — Eden side chips + Objects stub on DockRight.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_5_right_dock_side_chips.md
  4. .ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png
  5. apps/website/frontend/src/eden_chrome.rs DockRight
  6. apps/website/frontend/src/editor_ops.rs active_side / place_at

═══ PROBLEM ═══
  No side chips on right dock. Wire BLUFOR/OPFOR/INDFOR + Objects stub + search.
  Chips must drive active_side for place (.1). No F1–F6. No CIV.

═══ SHIPPED ═══
  T-180.1–.4 @ aeb51209 / 83557768 / 19acc593 / 63e7ef00
  active_side already in OpsCtx — chips must write it

═══ LOCKED ═══
  - Four chips: BLUFOR/OPFOR/INDFOR/Objects
  - Place side is hard gate (E4); catalog filter optional (E-L2b)
  - Objects = empty/coming soon, place no-op
  - No sandbag assets · no CIV · no F1–F6 row

═══ DO ═══
  1. Chip UI above search → set active_side / objects mode
  2. Objects empty-state
  3. Gates E1–E5 · .ai/artifacts/t180_5_verify_log.md · tag T-180.5

═══ DO NOT ═══
  Docs/registry · real props catalog · F1–F6 · CIV · Stitch ORBAT modal (.7)

═══ VERIFY ═══
  cargo test -p website-frontend --lib
  cargo test -p map-engine-core --features doc place_character_under_side_opfor
  make ci-local-leptos

═══ MANUAL ═══
  M-E1 eye-pass chips ref · M-E2 OPFOR place → red ring

═══ RETURN ═══
  SHA + tag T-180.5 · verify log · Ready for T-180.6
```
