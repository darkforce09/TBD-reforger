# T-152.20.1 verify log — wire the 4 unwired world-layer toggles to the engine

**Follow-up to T-152.20.** T-152.20 exposed all 12 `WorldClassToggles` in Mission Settings but
flagged that **4** of them — `roads`, `forest`, `contours`, `sea` — had **no engine consumer**
(visibility was purely zoom-LOD gated), so flipping them persisted the pref but did not change the
map. This slice wires those 4 so each toggle actually shows/hides its layer. Operator gate **O10**
("each pref off works") is now real for all 12 classes.

**Scope note:** T-152.20 was "TS dumb UI only". This wiring deliberately extends past that gate
(operator directive) — but stays **TS-only, zero `crates/**`**: every one of these 4 lanes already
decided its visibility in TS (a `class_visible(...)` / `forest_fill_effective` call whose result is
used TS-side to `upload_*(…, visible)` or `clear_vector_lane`). The fix ANDs the user toggle onto
that existing TS decision — no Rust visibility policy was moved into TS. Same pattern the already-wired
`airfield` lane uses (`wgpuWorldLoader.pushRoads`/`pushAirfieldApron` read `getClassToggles()`).

## Wiring (per key)
| key | lane(s) / role | gate site | reactivity |
|-----|----------------|-----------|------------|
| `sea` | ROLE_SEA (0) | `useWgpuDemVectors.pushSea`: `class_visible('sea',z) && getClassToggles().sea` | hook subscribes `subscribeWorldLayerPrefs` → `ctrl.sync` |
| `contours` | ROLE_CONTOURS (2) | `pushContours`: `class_visible('contour',z) && …contours` | same prefs subscription |
| `roads` | ROLE_ROADS (4) + ROLE_ROADS_CASING (3) | `wgpuWorldLoader.pushRoads`: `roadsOn` in band key + `visible = roadsOn && segment_count>0` on both lanes | `syncGlyphToggles` (effect dep `classToggles.roads`) resets band → recompose |
| `forest` | **both** ROLE_LANDCOVER (1) **and** ROLE_FOREST_FILL/OUTLINE (5/6) | landcover: `forest_fill_effective && …forest`; forest mass: `class_visible('forestFill'/'forestOutline',z) && …forest` | `syncGlyphToggles` re-pushes landcover (nulls `lastLandcoverVis`) + `WgpuForestMassController.resync()` (new); effect dep `classToggles.forest` |

`forest` gates two lanes on purpose — the landcover hulls are the low-zoom half of the same green
forest that the TBDD forest mass draws at high zoom (`forest_fill_effective` handoff), so "Forest
mass off" must hide both or the green never fully clears.

## Files (all TS)
- `wgpu/wgpuWorldLoader.ts` — `pushRoads` roads gate (band key + visible); `pushLandcover` `&& forest`; `syncGlyphToggles` now nulls `lastLandcoverVis` + calls `pushLandcover`.
- `wgpu/useWgpuForestMass.ts` — `pushComposite` `&& forest` on fill+outline; new public `resync()`.
- `wgpu/useWgpuDemVectors.ts` — `pushSea`/`pushContours` `&& sea`/`&& contours`; hook subscribes `subscribeWorldLayerPrefs` → `sync`.
- `WgpuTacticalMap.tsx` — toggle-sync effect: added deps `roads`, `forest`, `airfield` (**airfield was a pre-existing missing dep** — `set_airfield_toggle` only took effect on a camera move before); calls `forestControllerRef.current?.resync()`.

Untouched: `crates/**`, Rust/wasm, `worldLayerPrefs.ts`/`DEFAULT_TOGGLES`, `docs/**`, `.ai/tickets/**`.

## Verify (all exit 0)
```
cd apps/website/frontend
npm test    # 49 files, 365 tests PASS (no regression; class_visible parity scans unaffected)
npm run build   # tsc + vite OK
npm run lint    # eslint . — exit 0
cd ../../.. && git diff --stat crates/   # EMPTY
```

## Manual (operator, real GPU — WebGL2 harness can't verify visuals)
Settings → World layers — flip each of `roads`, `forest`, `contours`, `sea`: the corresponding
layer must show/hide, and (roads/forest) update without needing a pan; prefs survive reload. The
other 8 keys already reacted (T-152.20). This closes the O10 caveat for T-152.22 — all 12 classes
now operator-controllable end-to-end.

**M1 operator PASS — 2026-07-13.** All 12 world-layer toggles verified on real GPU: each layer
shows/hides on flip (incl. the newly-wired roads/forest/contours/sea), roads + forest update
without a pan, prefs survive reload. O10 controllability confirmed end-to-end for every class.
