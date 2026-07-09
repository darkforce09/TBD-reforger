# T-151.7.2 verify log — residual selection tint + zoom anchor

**Baseline:** tag **T-151.7.1** (`fa6ad959`).  
**Operator:** yellow ring + **SEL 0**; zoom repaired tint; wheel not grid-anchored.

---

## Root causes → fixes

| Bug | Cause | Fix |
|-----|--------|-----|
| **A** sticky yellow / SEL 0 | Detail path used per-row `patch_slot_lane` (skips / OOB / dragActive stuck); GPU ≠ store | **Full re-pack** from SoA + `selectedMask` on every selection change; dragActive invariant |
| **B** zoom drifts | React `viewState` lagged engine; pan `…viewState` clobbered wheel zoom; pick used stale ref | Engine SoT; **sync `viewStateRef` on wheel**; pan preserves live zoom; `getLiveViewState`; **abortPan** before wheel; getViewport from engine |

---

## What shipped

| File | Change |
|------|--------|
| `wgpu/wgpuSlots.ts` | `repackAndUploadDetailSlots`; syncSelection pure store re-materialize |
| `WgpuTacticalMap.tsx` | Camera SoT, wheel resize+zoom_at+sync ref, pan zoom merge, abortPan |
| `tools/useSelectTool.ts` | `getLiveViewState`, `abortPan` |
| `slotGpu.parity.test.ts` | +1 pan zoom merge contract test |

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| cargo fmt / clippy / tests | **PASS** |
| `npm test` | **PASS** — **393** (+1) |
| `npm run build` + `lint` | **PASS** |
| entry isolation | **PASS** |
| wasm | unchanged (no engine surface) |

---

## Manual (operator)

| ID | Check | Status |
|----|-------|--------|
| **S1** | Select → yellow + SEL 1; empty clear → all primary + SEL 0 **without zoom** | **operator** |
| **S2** | Rapid toggle / multi — tint matches SEL | **operator** |
| **S3** | Wheel + RMB-hold+wheel — world under cursor holds | **operator** |

Hard-refresh after pull (`?engine=wgpu`).

---

## Ready for Cursor doc sync
