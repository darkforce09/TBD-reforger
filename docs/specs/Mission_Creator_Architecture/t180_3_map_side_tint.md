# T-180.3 — Map side tint

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.  
**Verify log:** `.ai/artifacts/t180_3_verify_log.md`

## Problem (measured)

[`slots_gpu.rs:22-24`](../../../crates/map-engine-core/src/slots_gpu.rs):

```text
SLOT_PRIMARY_RGBA  = [173, 198, 255, 255]  // used for ALL unselected rings today
SLOT_SELECTED_RGBA = [250, 204,  21, 255]
```

Pack path applies primary to every non-selected slot. OPFOR/INDFOR look identical to BLUFOR.

## Locked

| ID | Decision |
|----|----------|
| C-L1 | Unselected ring tint from faction **key** (BLUFOR/OPFOR/INDFOR) — needs `FactionRow.key` from .1 |
| C-L2 | Selected still uses `SLOT_SELECTED_RGBA` |
| C-L3 | **Exact RGBA:** BLUFOR `[173,198,255,255]`, OPFOR `[248,113,113,255]`, INDFOR `[34,197,94,255]`, selected `[250,204,21,255]` |
| C-L4 | Unknown/missing side → BLUFOR tint |
| C-L5 | Tint in Rust pack — not CSS-only |

## File map

| File | Change |
|------|--------|
| `crates/map-engine-core/src/slots_gpu.rs` | `SIDE_*_RGBA` + pack takes per-instance side |
| SoA / GPU sync / FE bridge | Resolve slot→squad→faction.key→side for each ring |

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **C1** | `assert_eq!` exact triples above; pairwise ≠ | `side_tint_three_distinct` |
| **C2** | Selected ⇒ yellow regardless of side | `selected_overrides_side_tint` |
| **C3** | Pack 3 slots / 3 sides ⇒ 3 distinct tint u32 | `pack_rings_side_tints` |
| **C4** | Missing side → BLUFOR constant | `missing_side_defaults_blufor` |

## Verify

```bash
cargo test -p map-engine-core side_tint_three_distinct
cargo test -p map-engine-core selected_overrides_side_tint
cargo test -p map-engine-core pack_rings_side_tints
cargo test -p map-engine-core missing_side_defaults_blufor
# If pack path needs doc/SoA:
cargo test -p map-engine-core --features doc side_tint_ 2>/dev/null || true
cargo test -p website-frontend --lib
make ci-local-leptos
```

## Manual

M-C1: After .5 chips (or force `active_side`), one unit per side — three ring colors; select → yellow.

## Claude Code prompt — T-180.3 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-180.3** — Map side tint.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md
  3. docs/specs/Mission_Creator_Architecture/t180_3_map_side_tint.md
  4. crates/map-engine-core/src/slots_gpu.rs
  5. apps/website/frontend — GPU/slot pack call sites (resolve faction.key → side)

═══ PROBLEM ═══
  All unselected rings use SLOT_PRIMARY_RGBA. Need per-side tints from faction key.
  Selection yellow unchanged.

═══ SHIPPED ═══
  T-180.1 @ aeb51209 — FactionRow.key, place under side
  T-180.2 @ 83557768 — mutators/GC/vehicles

═══ LANGUAGE GATE ═══
  Pack/tint constants + pack path in map-engine-core (Rust).
  Leptos only passes side channel into existing pack/sync.

═══ LOCKED ═══
  - BLUFOR [173,198,255,255]
  - OPFOR  [248,113,113,255]
  - INDFOR [34,197,94,255]
  - Selected [250,204,21,255]
  - Missing side → BLUFOR
  - No squad lines (.4) · no CSS-only fake tint

═══ DO ═══
  1. SIDE_* constants + assert_eq in C1
  2. Thread side into pack_rings / sync from faction.key
  3. Tests C1–C4 · .ai/artifacts/t180_3_verify_log.md · tag T-180.3

═══ DO NOT ═══
  Docs/registry · T-180.4 lines · invent different RGBA · defer tint

═══ VERIFY ═══
  cargo test -p map-engine-core side_tint_three_distinct
  cargo test -p map-engine-core selected_overrides_side_tint
  cargo test -p map-engine-core pack_rings_side_tints
  cargo test -p map-engine-core missing_side_defaults_blufor
  cargo test -p website-frontend --lib
  make ci-local-leptos

═══ MANUAL ═══
  M-C1: three sides distinct rings; selection yellow

═══ RETURN ═══
  SHA + tag T-180.3 · verify log · Ready for T-180.4
```
