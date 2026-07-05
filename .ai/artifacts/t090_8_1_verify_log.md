# T-090.8.1 — Forest mass render · verify log

**Slice:** T-090.8.1 (Map Engine v2 slot 4 `world-landcover` + slot 8 `world-forest` / `world-forest-outline`)
**Spec:** `docs/specs/Mission_Creator_Architecture/t090_8_forest_vegetation_regions.md` · plan §4.2/§7 · LOD contract §N2/N3
**Date:** 2026-07-05 · **Executor:** claude-code

---

## What shipped

Two visual sources, per-slot per the t090_10 normative layer stack:

| Slot | Layer id | Source | Module |
|------|----------|--------|--------|
| 4 (before roads) | `world-landcover` | `objects/forest-regions.json.gz` Path B hulls (forest/field/waterBody), one pinned ~43 KB fetch | `worldmap/landCoverRegions.ts` |
| 8 (after buildings) | `world-forest` | TBDD `objects/density/{cx}_{cy}.bin` → marching squares **in the worker**, streamed by viewport | `worldmap/forestMass.ts` (pure) + `forestMassStore.ts` + `forestMassLayer.ts` |
| 8 | `world-forest-outline` | same marching-squares pass (iso contour segments) | same |

- **Marching squares** (A3 `DrawForestsNew` model): per-cell boundary walk, 16 cases, linear-interpolated crossings, saddle disambiguation by center average (deterministic — F6 vitest). Cell-local closed rings → `SolidPolygonLayer` binary (`_normalize:false`); contour segments → `LineLayer` binary (stride-interleaved src/tgt over one buffer). No global ring assembly, no per-instance JS objects end-to-end (worker → transferables → Deck binary data).
- **Worker extension:** `worldObjectsCore.loadForestMass(ids, iso?)` — TBDD fetch/decode cached worker-side as corner grids (≈0.4 MB max); geometry recomputed per delivery so transferred buffers can never detach the cache. `WorldManifestLite.densityPath` added. Rock channel decoded, unused (locked: styling deferred to P4).
- **Streaming:** `forestMassStore` mirrors chunkStore's factory/singleton shape, simpler by design — permanent per-chunk cache, no eviction (N11 P2b "pinned" policy; measured full-island composite 2.2 MB), composite rebuilt only on hydration commits (pan-stable `useSyncExternalStore` snapshot). Store early-exits on unchanged chunk sets.
- **Styling:** fill `rgba(34,120,60,α)` (locked); α ladder `forestFillAlpha` — 0.45 (z<−2.5), 0.35 (−2.5…+1), 0.12 (+1…+3, **latent**), 0 (>+3). Outline `rgba(24,90,45,0.9)` 1 px from −1.5 (no max). Land-cover tints: forest `rgba(46,90,50,0.15)` low-α underlay (hull+mass overlap can't double-darken), field tan / waterBody blue style-ready (zero such rows in the shipped export).
- **Hook wiring** (`useWorldMapLayers`): regions one-shot + `ensureForestStream` per terrain; `setForestViewport` per camera commit gated on the `forest` toggle; memo keys on derived band state (`forestAlpha`, gate booleans) — raw zoom never enters the memo. Flag OFF → `[]` before any work (R3/M-reg).

## Decisions (flag for doc sync)

1. **α above +1:** N3's `+1…+3 fill α 0.12` band conflicts with the shipped `classVisible('forestFill')` MAX gate (+1). The gate wins (single visibility authority, LOD5; lodGates untouchable this slice) — the 0.12 value is encoded in `forestFillAlpha` and activates only if the contract ever loosens `FOREST_FILL_MAX_ZOOM`.
2. **Layer order:** prompt's "(slots before roads per §4.2)" read as the **land-cover slot 4**; forest fill/outline sit at slot 8 after buildings — t090_10 §Layer stack + plan §4.2 + the hook's shipped header all agree.
3. **Land-cover gate:** no `landCover` class exists in lodGates; `world-landcover` rides `classVisible('forestFill')` + the `forest` toggle this slice (field/waterBody rows don't exist yet). Revisit when P4+ exports land them.
4. **Iso:** `DENSITY_ISO = 1` (plan §3.3 default, exported const). At iso=1 on integer corner counts, isolated count-1 corners collapse to zero area by interpolation — lone trees are not forest (regions export used threshold 2 for the same reason).
5. **Verifier gate 7 fix** (`packages/tbd-schema/scripts/verify-t090-spec-consistency.mjs`): pre-existing FAIL on main since the handoff commit `f216a081` — the t090_8 spec's embedded copy-paste prompt cites `npm run test/build/lint` under a `cd apps/website/frontend` line, and gate 7 resolved npm scripts against tbd-schema's package.json only (its line-scoped path exemption misses multi-line blocks). Fixed semantically: scripts now resolve against **both** package.jsons; a script missing from both still fails.
6. **Operator worktree state:** `.ai/tickets/registry.json` + `docs/TICKET_BRAINSTORM.md` carried uncommitted T-145/T-146 idea rows (operator's, another session) and `apps/mod/tbd-framework/resourceDatabase.rdb` was dirty — all deliberately left unstaged.

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `make schema-validate` | exit 0 (incl. the gate-7 fix; goldens untouched) |
| `npm run test -- --run forestMass landCoverRegions lodGates` | 5 files, 47 tests PASS |
| full vitest | **192/192** PASS (was 150 @ T-090.5.3 → +42) |
| `npm run build` | clean (tsc + vite; pre-existing chunk-size warning only) |
| `npm run lint` | clean (complexity ≤15 after marchCell/emitRing/gate-derivation extraction) |
| F3 (vitest) | `forestMassLayer.test.ts` — @ −2 `world-forest` visible fill α 0.35, `tree` gate closed, `building` open |
| LOD3 (vitest) | same file + `lodGates.test.ts` band pins (pre-existing, unmodified) |
| F6 (vitest) | byte-identical reproducibility on a 17×17 fixture |
| TBDD wire | round-trip vs a test-local encoder mirroring `density-grid.mjs` (16 B header, LE u16, offset-view safe) |

## Real-data smoke (all 625 Everon TBDD bins through the worker core, fs-backed)

```
268 forest chunks / 357 empty · 41,770 fill rings · 209,350 vertices ·
19,767 outline segments · 2.2 MB composite · 29 ms full-island decode+march
```

N11 P2b: 29 ms ≪ 3000 ms load budget; 2.2 MB geometry + ≤0.4 MB corner grids ≪ +20 MB. (Verify-only test, deleted before commit.)

## Manual (operator, `VITE_WORLDMAP_ENABLED=1 make web`, hard refresh) — PENDING

- [ ] F3: deckZoom −2 → green forest polygons visible, zero tree icons
- [ ] LOD3: −2 → forests polygons, trees hidden, buildings OBB unchanged
- [ ] Z-pan: forest mass stable while panning; pop-in ≤ buildings
- [ ] R5: FpsCounter ≥ 55 fps with forest layers on
- [ ] M-reg: flag OFF → first paint unchanged

## Files

New: `worldmap/forestMass.ts` (+test), `worldmap/forestMassLayer.ts` (+test), `worldmap/forestMassStore.ts` (+test), `worldmap/landCoverRegions.ts` (+test).
Modified: `worldmap/useWorldMapLayers.ts`, `worldmap/worldData.ts` (export `fetchGzJson`), `worldmap/chunkStore.test.ts` (manifest fixture field), `workers/worldObjectsCore.ts` (+test), `workers/worldObjects.worker.ts`, `workers/worldObjectsClient.ts`, `packages/tbd-schema/scripts/verify-t090-spec-consistency.mjs`.

**Ready for Cursor doc sync.**
