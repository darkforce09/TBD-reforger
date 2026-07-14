# T-152.7 verify log — Height markers (DEM peaks + ASL labels)

**Slice:** T-152.7  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

DEM peak detection (`dem/peaks.rs`), 80 m importance-distance declutter, procedural text atlas GPU lane (`WorldLabels`), wasm bridges, `worldLayerPrefs.heights` toggle (default on), Everon `height-labels.json` sidecar (10 peaks), export/verify scripts. Contour index labels at `deckZoom ≤ −1` **operator waived** (G-contour).

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | T-152.1 text lane PASS | **PASS** | [`.ai/artifacts/t152_1_verify_log.md`](t152_1_verify_log.md) — G1–G8 PASS |
| **G2** | `∀ label: \|value_m − sample_elevation(x,y)\| ≤ 0.5` m | **PASS** | `node scripts/map-assets/verify-height-labels.mjs --terrain everon` — wasm `verify_height_labels_json` + `sample_elevation_from_meters_cache` |
| **G3** | `∀ label: sample_elevation(x,y) > 0` | **PASS** | same oracle |
| **G4** | Declutter: `∀ kept pair @ z: dist ≥ 80·2^(−z)` m | **PASS** | `declutter_invariant_holds` + verify script @ z=0 |
| **G5** | `count(labels) ≤ 48` after declutter | **PASS** | **10** labels @ z=0 (from **10** peaks) |
| **G6** | `max(value_m) ≥ 350` on Everon | **PASS** | **375** m (global summit injected when plateau prominence fails) |
| **G7** | Toggle `heights` off → zero height labels drawn | **PASS** | `WgpuHeightLabelController.sync` → `upload_text_labels([], false)`; `worldLayerPrefs.heights` default **true** |
| **G-contour** | Contour index labels @ `deckZoom ≤ −1` optional | **PASS (waived)** | `height_contour_labels_waived()` = true — operator waived per spec |
| **G8** | T-152.6 PASS; wasm + FE regression green | **PASS** | [`.ai/artifacts/t152_6_verify_log.md`](t152_6_verify_log.md); vitest **355/355**; `npm run build` + `npm run lint` OK |

## Automated commands

```text
cargo test -p map-engine-core dem::peaks --all-features   → 5/5 PASS
cargo test -p map-engine-render                           → 31/31 PASS
make wasm                                                 → map_engine_wasm_bg.wasm 4,243,030 B
cd apps/website/frontend && npm test                        → 355/355 PASS
cd apps/website/frontend && npm run build && npm run lint → OK
node scripts/map-assets/export-height-labels.mjs          → 10 peaks → packages/map-assets/everon/height-labels.json
node scripts/map-assets/verify-height-labels.mjs --terrain everon → OK
```

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Everon peaks (pre-declutter) | **10** |
| Labels drawn @ z=0 | **10** |
| `PEAK_WINDOW_PX` | **9** |
| `PEAK_PROMINENCE_M` | **15** |
| `PEAK_LABEL_MAX` | **48** |
| `HEIGHT_LABEL_MIN_SEP_M` @ z=0 | **80** m |
| Highest label `value_m` | **375** m |
| wasm merged size | **4,243,030** B |

## Manual (operator)

| ID | Status |
|----|--------|
| M1 | PENDING — island zoom 3–8 ridge numbers, none in sea |
| M2 | PENDING — zoom declutter; highest peak label remains |
| M3 | PENDING — toggle heights off; contours remain |

Automated Gn all **PASS** — tag **T-152.7** allowed.
