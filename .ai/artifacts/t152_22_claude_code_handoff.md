# T-152.22 — Close-out handoff (automated half + operator sheet)

**Slice:** T-152.22 · **Executor:** human (operator signs) · Claude assists G1–G4 only  
**Branch:** `ticket/T-152` · **Tag:** `T-152.22`  
**Spec:** [`t152_22_e2e_regate_operator.md`](../docs/specs/Mission_Creator_Architecture/t152_22_e2e_regate_operator.md)

## Operator decision (2026-07-14)

**T-152.18 / T-152.19 deferred indefinitely — do NOT block merge.** Redraw icons + Path B label sidecars ship as-is.

## Claude scope (automated only)

1. Run regression sweep at tip (`.12–.21` verify commands + `make ci-local`)
2. Extend/run de-vacuous master if scripts exist; else document command block in verify log
3. Pre-fill `t152_22_verify_log.md` G1–G4 tables — **leave O1–O12 blank for operator**
4. Create `.ai/artifacts/t152_22_operator/` + README
5. Commit verify log + any gate script additions; tag **T-152.22**
6. **Do NOT** sign O-rows, merge, or `./scripts/ticket done`

## Operator scope (you)

1. Browser: Mission Creator Everon, Map basemap, default zoom
2. Walk **O1–O12** (see verify log) — PASS/FAIL each; screenshot per row → `t152_22_operator/`
3. Record **M2 merge go** in verify log
4. Merge `ticket/T-152` → `main` when satisfied

## O-checklist quick reference

| Row | Check |
|-----|-------|
| O1 | Map loads, no blank/panic |
| O2 | Fences readable @ z≈1.5+ |
| O3 | Harbor pier strips (Saint-Philippe) |
| O4 | Bridge deck + rails |
| O5 | Airfield apron + runway |
| O6 | Hangar/tower icons @ airfield |
| O7 | Height labels on ridges; none in sea |
| O8 | Town names @ island zoom (Gorey, Morton) |
| O9 | Highway name on a curve |
| O10 | All 12 layer toggles flip layers |
| O11 | Pan/zoom ≥55 fps @ default |
| O12 | Satellite ↔ Map switch |

**O2 note:** criterion is z≥**1.5** (T-152.15), not z≥3.
