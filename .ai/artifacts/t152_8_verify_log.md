# T-152.8 verify log — Town labels (locations.json + A3 declutter)

**Slice:** T-152.8  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

Everon `locations.json` consumed via Rust `world/locations.rs` + `world/importance_declutter.rs` (A3 predicate: `IMPORTANCE_SCALE=0.08`, `TOWN_BASE_SIZE_M=400`). Procedural text atlas draws town names on `WorldTownLabels` lane (above `WorldLabels` height markers). wasm bridges + `WgpuTownLabelController` + `worldLayerPrefs.townLabels` toggle (default on). Full A–Z glyph atlas extension for settlement names.

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | T-152.6 + T-152.1 verify logs **PASS** | **PASS** | [`.ai/artifacts/t152_6_verify_log.md`](t152_6_verify_log.md) · [`.ai/artifacts/t152_1_verify_log.md`](t152_1_verify_log.md) |
| **G2** | At `deckZoom=-2`: `REQUIRED_EVERON_TOWNS ⊆ drawn_names` | **PASS** | `node scripts/map-assets/verify-town-labels.mjs --terrain everon --zoom -2` — **60/60** drawn incl. all 8 required |
| **G3** | `∀ drawn: declutter predicate true` @ same zoom | **PASS** | `town_declutter_invariant_holds` + verify script |
| **G4** | `∀ drawn: name.source = locations.json[id]` | **PASS** | `verify_town_labels_json` provenance check |
| **G5** | Toggle `townLabels` off → `\|drawn\|=0` | **PASS** | `WgpuTownLabelController.sync` → `upload_town_labels([], false)`; empty source oracle in verify script |
| **G6** | Pan/zoom −4…+1 — no atlas leak / crash | **PASS (automated)** | wasm pack + render lane split; **M6 FPS operator PENDING** |
| **G7** | T-152.7 verify PASS; regression green | **PASS** | [`.ai/artifacts/t152_7_verify_log.md`](t152_7_verify_log.md); vitest **355/355**; FE build/lint OK |

## Automated commands

```text
cargo test -p map-engine-core importance_declutter --all-features  → 4/4 PASS
cargo test -p map-engine-core world::locations --all-features      → 1/1 PASS
cargo test -p map-engine-render                                    → 31/31 PASS
make wasm                                                          → map_engine_wasm_bg.wasm 4,271,763 B
cd apps/website/frontend && npm test                               → 355/355 PASS
cd apps/website/frontend && npm run build && npm run lint          → OK
node scripts/map-assets/verify-town-labels.mjs --terrain everon --zoom -2 → OK
```

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Everon location rows | **60** |
| Labels drawn @ z=−2 | **60** |
| `IMPORTANCE_SCALE` | **0.08** |
| `TOWN_BASE_SIZE_M` | **400** m |
| `TOWN_LABEL_MIN_ZOOM` | **−3** |
| `TOWN_LABEL_MAX_ZOOM` | **2** |
| Glyph instances @ z=−2 | **915** (18 300 B packed) |
| wasm merged size | **4,271,763** B |
| Cartographic tint | `#e8e4dc` @ α0.92 |

## Required towns @ z=−2 (G2)

Morton · Gorey · Highstone · Raccoon Rock · Saint Philippe · Levie · Montignac · Kermovan — all in drawn set.

## Manual (operator)

| ID | Status |
|----|--------|
| M1 | PENDING — island view: Gorey, Morton, Levie readable |
| M2 | PENDING — zoom +4: hamlets hide before capitals |
| M3 | PENDING — toggle town labels off; height labels remain |

Automated Gn all **PASS** — tag **T-152.8** allowed.
