# T-180.5 — Right dock Eden side chips (absorbs T-074)

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Visual ref:** [`.ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png`](../../../.ai/artifacts/t180_stitch_orbat_modal/eden_side_chips_ref.png)  
**Verify log:** `.ai/artifacts/t180_5_verify_log.md`

## Problem (measured)

[`DockRight`](../../../apps/website/frontend/src/eden_chrome.rs) (~1234): tabs Factions / Vehicles / Markers + search. **No** BLUFOR/OPFOR/INDFOR/Objects chips. `active_side` from T-180.1 cannot be set from UI. Gap `RIGHT-SUBMODE-001` / old T-074.

## Locked

| ID | Decision |
|----|----------|
| E-L1 | Chip row: **blue BLUFOR, red OPFOR, green INDFOR, yellow Objects** — **no F1–F6**, **no CIV** |
| E-L2 | Units mode (side chip): drives `OpsCtx.active_side` (from .1) — place uses that string exactly `"BLUFOR"\|"OPFOR"\|"INDFOR"` |
| E-L2b | Catalog filter: until library-driven palette is SoT, filter may be **side chip → active_side only** (registry characters are not reliably side-tagged). Document in verify log if catalog is unfiltered but place side is correct — **place side is the hard gate (E4)** |
| E-L3 | Objects mode: empty / “Objects coming soon (sandbags/props)” — no panic; place disabled or no-op |
| E-L4 | Existing Factions/Vehicles/Markers tabs: keep or fold — **side chips sit above search**; Vehicles tab still stub until .8 if needed |
| E-L5 | Search still filters visible catalog |

## File map

| File | Change |
|------|--------|
| `apps/website/frontend/src/eden_chrome.rs` | Chip UI + mode state |
| `apps/website/frontend/src/asset_catalog.rs` | Filter by side; Objects empty |
| `apps/website/frontend/src/mission_editor.rs` | `active_side` already from .1 — chips write it |

## Class-R gates

| ID | Assert | How |
|----|--------|-----|
| **E1** | Chip markers present; no F1–F6 function-key mode row in DockRight | `rg` + test |
| **E2** | Click OPFOR ⇒ `active_side == OPFOR` | FE test |
| **E3** | Objects mode ⇒ empty-state string visible | FE test |
| **E4** | After OPFOR selected, `place_at` mints OPFOR faction (integration with .1) | ops/core test |
| **E5** | CIV not in chip row | `rg` / assert 3 combat + objects only |

## Verify

```bash
make ci-local-leptos
rg -n 'F1|F2|F3|F4|F5|F6' apps/website/frontend/src/eden_chrome.rs | head
```

## Manual

M-E1: Eye-pass vs `eden_side_chips_ref.png` (middle row colors + search; ignore top F-keys).  
M-E2: BLUFOR place → blue ring (.3); OPFOR place → red.

## Claude Code prompt — T-180.5 (copy-paste)

```
Read CLAUDE.md first.
Implement **T-180.5** — Eden side chips + Objects stub on DockRight.

═══ READ ═══
  t180_5_right_dock_side_chips.md · hub · handoff
  eden_side_chips_ref.png · eden_chrome.rs DockRight · asset_catalog.rs

═══ PROBLEM ═══
  No side chips. Wire BLUFOR/OPFOR/INDFOR + Objects stub + search. No F1–F6. No CIV.

═══ DO ═══
  1. Chip UI → active_side
  2. Catalog filter / Objects empty
  3. Gates E1–E5 · tag T-180.5

═══ DO NOT ═══
  Docs · real sandbag assets · F1–F6 · CIV chip

═══ VERIFY ═══
  make ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.5 · Ready for T-180.6
```
