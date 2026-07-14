# T-152 — Merge readiness (program close-out)

**Branch:** `ticket/T-152` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152`  
**Last code tag:** `T-152.21` @ `d5c746df`  
**Close-out:** **T-152.22** — **GO to merge** (operator 2026-07-14)

## Merge gate

| # | Gate | Status |
|---|------|--------|
| 1 | Remediation **.12–.21** shipped + tagged | **DONE** |
| 2 | **T-152.22** automated G1–G4 | **PASS (waived)** — see verify log quotes |
| 3 | Operator **O1–O12** | **PASS** (bulk "good enough"; screenshot pack waived) |
| 4 | `t152_22_verify_log.md` + tag **T-152.22** | This close-out commit |
| 5 | Operator **merge go** (M2) | **GO** 2026-07-14 |

## Explicitly NOT blocking merge

| Slice | Reason |
|-------|--------|
| **T-152.18** | Icon Reforger extract — deferred; T-152.2 redraw atlas retained |
| **T-152.19** | Workbench Path A label/road export — deferred; Path B sidecars retained |

## Shipped remediation summary (.12–.21)

| Tag | What |
|-----|------|
| T-152.12 | Text lane alive + upright |
| T-152.13 / .13.1 | Spleen atlas + halo |
| T-152.14 / .14.1 | Tree budget + glyph atlas fix |
| T-152.15 | Fences/piers/bridges |
| T-152.16 | Height markers credible |
| T-152.17 | Town labels settlement-only |
| T-152.20 / .20.1 | 12/12 layer toggles wired |
| T-152.21 | Landmark badges @ default zoom |

## Merge procedure (operator — do now)

```bash
# From main repo checkout (not necessarily this worktree):
cd /home/Samuel/Projects/TBD-Reforger
git fetch origin   # if needed
git checkout main
git merge ticket/T-152 -m "Merge ticket/T-152: map cartographic fidelity"
# optional: make ci-local
./scripts/ticket done T-152
# then ask Cursor: post-merge doc sync (hub complete, CLAUDE §Status)
```

**After merge:** Cursor sets registry `T-152 → shipped`, hub **complete**, `./scripts/ticket sync`.

## Known acceptable gaps (good-enough ship)

- Curated road names (6 majors) — `.19` deferred
- Redraw landmark icons — `.18` deferred
- Contour index labels — fresh waiver in `.16` verify log
- Perfect fence/field continuity — data limitation
- Formal `.22` screenshot pack + extended master re-suite — waived by operator merge go
