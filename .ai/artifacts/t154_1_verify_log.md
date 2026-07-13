# T-154.1 verify log — Doll polish (contrast, rotation, hover + callout)

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode D session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t154_1_doll_polish.md` ·
**Baseline:** `1a9a4a59` (T-154.0 docs-sync)

## LOC proof (D5)

| file | LOC | delta vs .0 | note |
|------|----:|------------:|------|
| `loadout/SoldierModel3D.tsx` | 264 | +101 | hover tooltip + callout chip/leader DOM + rAF anchor tracking — the allowed React/DOM lane; zero policy (pick/hover/anchor/colors all Rust) |
| `loadout/dollEngine.ts` | 21 | 0 | unchanged |

## Automated gates — ALL PASS

| gate | result |
|------|--------|
| `make wasm-ci` | exit 0 — core **73** (new: hover lift distinct per state · every region anchor inside 800×600 @ yaw 0 + unknown-region None · anchor ≡ transform_vector projection < 1e-9 px) · render **29** (new: hover flip rewrites exactly the hovered region's instance colors) |
| `make wasm` | `map_engine_wasm_bg.wasm` = **4,217,176 B** (T-154.0: 4,216,072 → +1,104) |
| `npx vitest run` | **358/358** (doll parity suite unchanged — region contract + pick goldens unaffected by sign/color changes: pick math has no yaw-sign dependence at yaw 0/π goldens) |
| `npm run build` + `tsc --noEmit` | clean |
| `npm run lint` | pre-existing `router.tsx` only (one new `react-hooks/refs` finding during dev — ref write moved into an effect) |
| `node scripts/website/verify-wgpu-gpu.mjs` | exit 0 — **36/36 probes, allPass true**, incl. `doll` (background probe auto-derived the new clear bytes [133,143,161]) |

## Manual (operator) — the Mode D pause

| # | check |
|---|-------|
| S1 | Doll clearly visible: dark gear on the mid-light grey-blue backdrop |
| S2 | Drag LEFT → character turns LEFT (drag = turn, not camera orbit) |
| S3 | Hover any part → tone lifts + cursor chip "{Region} — {item / empty}"; leaves cleanly on pointer-out; drops while dragging |
| S4 | Active part: pinned name chip + thin leader line to the part; tracks smoothly while rotating; hides when the part rotates behind the soldier |
| S5 | Click-to-select, rail sync, states, weight readout, Export — .10.8/.154.0 regression unchanged |
| S6 | `/_spike/wgpu` doll thumbnail now light-backed; self-check `doll` PASS in the report |

## Out of scope (locked)

Pitch/zoom orbit (operator: not needed) · mesh fidelity (explained: shapelier soldier —
bevels, dome helmet, real rifle silhouette — separate slice when asked) · hover on decor.
