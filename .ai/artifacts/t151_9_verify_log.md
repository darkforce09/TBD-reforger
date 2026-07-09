# T-151.9 verify log — Deck flip + retirement

**Tag:** `T-151.9`  
**Baseline:** T-151.8.1 (`ec59d10e`) · docs sync @ `c52d1fc8`  
**Worktree:** `tbd-reforger-wgpu-spike/`  
**Date:** 2026-07-09

## Scope shipped

1. Mission Creator **always** mounts `WgpuTacticalMap` (no `?engine=`, no `VITE_MC_ENGINE` branch).
2. Deck runtime deleted (TacticalMap, layer hooks, worldmap `*Layer`/stores, worker trio, hybrids, `viewportBbox`, DocCoreSpike).
3. Deck-free JS oracle `_wasm/oracles/jsWorldChunkOracle.ts` + residency goldens (22 steps).
4. `satelliteUnified`: `parseTbdSat` + `pickBaseLevel` only (luma `loadUnifiedSatTexture` removed).
5. Six deck/luma packages moved `dependencies` → `devDependencies`.
6. `FpsCounter` off Deck glyph stream (rAF FPS + zoom only).

LANGUAGE GATE: `wgpuSlots.ts` = **56** LOC (≤ 60). No new cull/LOD policy in TS.

---

## Class R/S gates

| Gate | Result |
|------|--------|
| Vitest `N` | **281** = `393 − 112 + 0` (`M=0`) PASS |
| cargo fmt / clippy / tests | PASS |
| `make wasm` | PASS |
| `npm run build` / `lint` | PASS |
| `wgpuSlots.ts` ≤ 60 | **56** PASS |
| deck/luma only in `devDependencies` | node assert PASS |
| `dist/assets` free of `@deck.gl` / `DeckGL` / `@luma.gl` | PASS |
| `map_engine_wasm` in dist | PASS |
| no `map_engine_wasm_bg` in `index-*.js` | PASS |
| DELETE paths absent | PASS |
| Deck importers allowlist (3 paths) | PASS |
| Source prod paths Deck-free | PASS |

### Bundle ledger (`dist/assets`)

| Metric | Before (pre-delete tree on disk) | After T-151.9 |
|--------|----------------------------------|---------------|
| `du -sb dist/assets` | 7 151 199 | **6 273 423** |
| `MissionCreatorPage-*.js` | (prior build) | 103 945 |
| `WgpuTacticalMap-*.js` | — | 40 350 |
| `index-*.js` | — | 405 829 |

Delta: **−877 776** bytes (~12% smaller assets dir vs last local dist).

---

## Manual (operator)

| ID | Status |
|----|--------|
| S1 | Code gate: empty-query edit mounts wgpu only — **PASS** (code). Operator pan/zoom confirm recommended. |
| S2 | Place/select/drag + Save 201 + Export — **operator** |
| S3 | ~367k IDB+server + conflict — **operator** |
| S4 | Bundle ledger above — **PASS** |
| S5 | Dist Class R — **PASS** |

---

## Artifacts

- Oracle: `apps/website/frontend/src/features/_wasm/oracles/jsWorldChunkOracle.ts`
- Goldens: `apps/website/frontend/src/features/_wasm/oracles/goldens/residency_everon_v1.json` (baseline `ec59d10e`, 22 steps)

## Ready for Cursor doc sync

Registry `T-151.9 → shipped`; hub W9; CLAUDE next → W10 / T-069; `./scripts/ticket sync`.
