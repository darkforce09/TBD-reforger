# T-152.20 verify log — Mission Settings world-layer toggle completeness

**Slice:** T-152.20 · **Spec:** `docs/specs/Mission_Creator_Architecture/t152_20_settings_completeness.md`
· **Handoff:** `.ai/artifacts/t152_20_claude_code_handoff.md`
**Language gate:** TS dumb UI only — zero `crates/**`, zero Rust/wasm, zero engine/default change.

## What shipped
Mission Settings → **World layers** now exposes a control for **all 12** `WorldClassToggles`
keys (was 5). Added the 7 previously UI-hidden toggles **before** the existing 5, in locked order:

| # | Label | key | default |
|---|-------|-----|---------|
| 1 | Roads | `roads` | on |
| 2 | Buildings | `buildings` | on |
| 3 | Forest mass | `forest` | on |
| 4 | Trees | `trees` | on |
| 5 | Props | `props` | **off** |
| 6 | Contours | `contours` | on |
| 7 | Sea | `sea` | on |
| 8–12 | Fences / Airfield / Height labels / Town labels / Road names | *(existing, untouched)* | on |

`DEFAULT_TOGGLES` is byte-identical (`props` stays `false`, only now flippable). Existing 5 rows
untouched. Toggles use the existing `setClassToggle(<key>, on)` + `useClassToggles()` plumbing.

## Files
- **Edit** `apps/website/frontend/src/features/mission-creator/layout/MissionSettingsDialog.tsx` — 7 `ToggleField` rows + manifest import.
- **New** `apps/website/frontend/src/features/mission-creator/layout/worldLayerFields.ts` — `WORLD_LAYER_TOGGLE_LABELS satisfies Record<keyof WorldClassToggles, string>` (compile-time keyof completeness guard).
- **New** `apps/website/frontend/src/features/mission-creator/layout/worldLayerFields.test.ts` — completeness test.
- **Edit** `apps/website/frontend/src/features/tactical-map/state/worldLayerPrefs.test.ts` — 7 persistence cases (`it.each`).

Untouched: `worldLayerPrefs.ts` (incl. `DEFAULT_TOGGLES`), `wgpuWorldLoader.ts`, `crates/**`, `docs/**`, `.ai/tickets/**`.

## Completeness test (G1 / L4 — future-proof, 12/12)
- **File:** `worldLayerFields.test.ts` · **describe:** `MissionSettingsDialog world-layer completeness (T-152.20 / A15 / O10)`
- **12/12 test name:** `exposes a control for every WorldClassToggles key (12/12)`
- Two lines of defence:
  1. **compile-time** — `satisfies Record<keyof WorldClassToggles, string>` in `worldLayerFields.ts`: adding a 13th class to the interface without a label here fails `tsc` (`npm run build`).
  2. **runtime** — manifest key set must equal `Object.keys(getClassToggles())` (the keyof-driven enumerator; `DEFAULT_TOGGLES` is byte-locked to the interface by the existing defaults test). A class added to `WorldClassToggles` + `DEFAULT_TOGGLES` but not exposed here fails this assertion.

## Verify (all exit 0)
```
cd apps/website/frontend
npm ci --no-audit                # preflight — exit 0
npm test                         # 49 files, 365 tests PASS (incl. 2 completeness + 7 persistence)
npm run build                    # tsc + vite — built OK (satisfies guard compiled)
npm run lint                     # eslint . — exit 0
cd ../../.. && git diff --stat crates/    # EMPTY (G3)
```
- `npm test` → **Test Files 49 passed (49) · Tests 365 passed (365)**.
- New: `worldLayerFields.test.ts` (2) + `worldLayerPrefs.test.ts` +7 `it.each` rows (roads/buildings/forest/trees/props/contours/sea — each flipped to its non-default value, round-tripped through `getClassToggles()` + `localStorage['tbd-mc-world-layers']`, survives reload via `importFresh()`).
- `git diff --stat crates/` → empty. **G3 PASS.**

## Manual M1 (operator, browser)
Settings → World layers — flip each of the 12; prefs survive reload.

**Engine-reaction caveat (recorded for O10 in T-152.22):** of the 12 keys, **8 are engine-wired**
so the map visibly redraws on toggle — `trees`/`props`/`buildings`/`fences`/`airfield`
(`wgpu/wgpuWorldLoader.ts:131-133`), `heights`/`townLabels`/`roadNames`
(`useWgpuHeightLabels`/`useWgpuTownLabels`/`useWgpuRoadLabels`). The remaining **4**
— `roads`, `forest`, `contours`, `sea` — have **no toggle consumer today** (visibility is
zoom-LOD gated, e.g. `class_visible('sea', zoom)`), so they **persist correctly but do not visibly
redraw** when flipped. That is engine visibility policy owned by T-152.14/.15 and is out of scope
for this UI-only slice; this slice makes the prefs operator-controllable and O10 executable. Flag
raised so T-152.22 O10 either wires those 4 or scopes its pass to the 8 wired classes.
