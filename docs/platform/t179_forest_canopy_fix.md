# T-179 — Fix blocky / holed density canopy

**Status:** `done` · **Tag:** **T-179** · **Executor:** Cursor/Grok (operator override) · **Branch:** `main`  
**Depends on:** T-178 (shipped)  
**Scope:** `apps/website/frontend/src/world_assets/**`, `crates/map-engine-{core,render}/**`, `tools/tbd-tools/src/smokes.rs`. **Not** `apps/mod/`. **Not** T-071.1.  
**Verify:** [`.ai/artifacts/t179_verify_log.md`](../../.ai/artifacts/t179_verify_log.md)

## Why

T-178’s island density lifecycle was correct (one fetch → stitch → single GPU commit) but geometry quality was wrong:

1. Nearest + hard iso → Minecraft 8 m cells  
2. Outline was a fake `segments=0|1` + `fwidth` rim, not MS polylines  
3. Failed density bins silently zeroed → 512 m holes while still marking uploaded  

## Must-ship

| ID | Done when |
|----|-----------|
| A | Retry failed bins; arm only at `bins_ok === 625`; bridge + fullmap equality |
| B | Linear soft fill + corner UVs + fwidth AA iso; hard Nearest path gone |
| C | One-shot MS hairlines role 6; real `forest_outline_segments` |
| D | Smokes rewritten; `make leptos-gates` + `make ci-local`; verify log |

## Explicit out of scope

- 32 m landcover forest wash  
- Progressive `push_composite` during fetch  
- Full-island MS fill triangulation (soft shader fill retained)  
- `CANOPY_MASS_ISO` retune unless operator G-A still fails after A–C  

## Claude Code / Cursor prompt (shipped)

Implement A→B→C→D per plan; no deferrals; Class-R pins 1601 / 625 / outline ≥ measured floor.
