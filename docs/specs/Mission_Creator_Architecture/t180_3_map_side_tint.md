# T-180.3 — Map side tint

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1 · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.  
**Verify log:** `.ai/artifacts/t180_3_verify_log.md`

## Problem (measured)

[`slots_gpu.rs:22-24`](../../../crates/map-engine-core/src/slots_gpu.rs):

```text
SLOT_PRIMARY_RGBA  = [173, 198, 255, 255]  // Aegis primary — used for ALL unselected rings
SLOT_SELECTED_RGBA = [250, 204,  21, 255]  // tactical yellow
```

Pack path ([`slots_gpu.rs:83-90`](../../../crates/map-engine-core/src/slots_gpu.rs)) applies primary to every non-selected slot. OPFOR/INDFOR are visually identical to BLUFOR.

## Locked

| ID | Decision |
|----|----------|
| C-L1 | Unselected ring tint from faction **side key** (BLUFOR/OPFOR/INDFOR) |
| C-L2 | Selected still uses `SLOT_SELECTED_RGBA` (yellow wins over side) |
| C-L3 | **Exact RGBA (locked in [`t180_class_r_pins.md`](t180_class_r_pins.md)):** BLUFOR `[173,198,255,255]`, OPFOR `[248,113,113,255]` (`#f87171`), INDFOR `[34,197,94,255]` (`#22c55e`), selected `[250,204,21,255]` |
| C-L4 | Unknown/missing side → BLUFOR tint (not crash) |
| C-L5 | Tint math in Rust pack — not CSS-only overlay that GPU ignores |

## File map

| File | Change |
|------|--------|
| `crates/map-engine-core/src/slots_gpu.rs` | `SIDE_*_RGBA` + pack takes per-instance side |
| SoA / GPU sync | Resolve slot→squad→faction.key→side enum each pack |
| Call sites | Update all pack_* that emit slot rings |

## Class-R gates

| ID | Assert | Test |
|----|--------|------|
| **C1** | Constants equal exact triples above; pairwise ≠ | `side_tint_three_distinct` — `assert_eq!(SIDE_BLUFOR_RGBA, [173,198,255,255])` etc. |
| **C2** | Selected mask ⇒ yellow regardless of side | `selected_overrides_side_tint` |
| **C3** | Pack 3 slots / 3 sides ⇒ 3 distinct tint u32 in bytes | `pack_rings_side_tints` |
| **C4** | Missing side → BLUFOR constant | `missing_side_defaults_blufor` |

## Verify

```bash
cargo test -p map-engine-core side_tint_three_distinct
cargo test -p map-engine-core selected_overrides_side_tint
cargo test -p map-engine-core pack_rings_side_tints
cargo test -p map-engine-core missing_side_defaults_blufor
make ci-local-leptos
```

## Manual

M-C1: Place one unit per side (after .5 chips) — three distinct ring colors; select one → yellow.

## Claude Code prompt — T-180.3 (copy-paste)

```
Read CLAUDE.md first.
Implement **T-180.3** — Map side tint.

═══ READ ═══
  t180_3_map_side_tint.md · hub · handoff · slots_gpu.rs

═══ PROBLEM ═══
  All rings share SLOT_PRIMARY_RGBA. Need per-side tints; selection yellow unchanged.

═══ LANGUAGE GATE ═══
  Pack/tint in Rust slots_gpu + SoA sync.

═══ DO ═══
  1. SIDE_* constants (document RGBA in verify log)
  2. Thread side into pack
  3. Tests C1–C4 · tag T-180.3

═══ DO NOT ═══
  Docs · squad lines (.4) · CSS-only fake tint

═══ VERIFY ═══
  cargo test -p map-engine-core side_tint_ selected_overrides pack_rings_side missing_side
  make ci-local-leptos

═══ RETURN ═══
  SHA + tag T-180.3 · Ready for T-180.4
```
