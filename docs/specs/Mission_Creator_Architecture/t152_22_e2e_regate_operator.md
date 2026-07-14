# T-152.22 — E2E re-gate (de-vacuoused) + operator O1–O12 sign-off

**Ticket:** T-152 · **Slice:** T-152.22 (remediation ladder #11 — program close-out)
**Status:** `shipped` (operator GO 2026-07-14 — see verify log)
**Executor:** **human** (operator) — Claude Code assists with the automated half, cannot self-sign
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §6.5 (A14, A16, D12) · [`t152_10_e2e_cartographic_gate.md`](t152_10_e2e_cartographic_gate.md)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.22`**
**Depends on:** T-152.12–.21 all shipped (every remediation Gn PASS)

## In one sentence

Re-run the program-wide gate with the vacuity holes closed (pier census > 0, GPU text upright, tree never-blank property, waiver-quote rule), then the operator signs O1–O12 with screenshots — only after this may T-152 merge to `main` and be marked done.

---

## Problem

Audit §6.5: T-152.10's automated master passed while G4 (piers) was vacuous, G-contour silently inherited a waiver, no gate ever exercised the GPU text path, and **all** O1–O12 operator rows stayed PENDING with no screenshot directory ever created. "Automated PASS" and "operator-visible" diverged program-wide — this slice re-couples them.

---

## Goal

1. **De-vacuoused automated master:** extend the .10 gate suite with: anti-vacuous census clauses (pier strips ≥ 0.99×2,299; fence strips > 30,000; every census gate FAILS on empty set); the .12 GPU text gates (pipeline smoke + upright readback, both backends); the .14 tree never-blank zoom-ladder property; the .16 band + floor gates; a **waiver-quote rule** — any `PASS (waived)` row must embed a dated operator quote from this remediation cycle, else FAIL.
2. **Regression sweep:** every T-152.12–.21 verify log Gn re-confirmed at tip (re-run commands, not trust logs); full CI-local suites.
3. **Operator O1–O12:** the .10 checklist re-run in the browser against the remediated tip; each row signed PASS/FAIL in `.ai/artifacts/t152_22_verify_log.md`; screenshots under `.ai/artifacts/t152_22_operator/` (dir actually created this time — one image per O-row minimum).
4. **Program close:** on all-green, hub flips to "complete pending merge"; merge to `main` + `./scripts/ticket done T-152` remain operator calls after sign-off (documented as next steps, not executed by the agent).

---

## Out of scope

- New features/fixes (any FAIL spawns a T-152.2x hotfix slice — filed, not patched inline here).
- Registry/doc sync (Cursor after sign-off).
- Merge itself.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Operator signs; agent may pre-fill evidence but every O-row needs the human mark (per .10 L3) | A16 |
| L2 | Anti-vacuous clause is structural: census gates assert non-empty domain **before** the ∀ predicate | A14 |
| L3 | Waiver rule: `PASS (waived)` valid only with fresh dated operator quote in this slice's log | No inherited waivers |
| L4 | Any O-row FAIL ⇒ file `T-152.2x` hotfix slice (registry row + spec stub) before program close — no silent deferral | Audit L8 discipline |
| L5 | Screenshot dir `.ai/artifacts/t152_22_operator/` committed (LFS if large) — ≥ 12 images (one per O-row) | Evidence exists this time |
| L6 | Commit `T-152.22:` · tag `T-152.22` · verify log | House convention |

---

## O-checklist (inherited from `t152_10_e2e_cartographic_gate.md:55-68`, re-run verbatim)

O1 map loads (no blank/panic) · O2 fences visible @ new gate (≥ 1.5 per .15) · O3 pier thin strip @ harbor · O4 bridge deck + rail · O5 airfield apron + runway · O6 hangar/tower icons · O7 height labels on ridges, none in sea · O8 town names @ island zoom (Gorey, Morton readable) · O9 highway name on curve · O10 layer toggles — **all 12** prefs off/on work (.20) · O11 pan/zoom ≥ 55 fps @ default zoom · O12 Satellite ↔ Map switch sane.

*(O2's zoom criterion updates from "≥3" to the .15 locked gate — note the delta in the log.)*

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Extended master suite exit 0: .10 gates + .12 GPU text + .14 property + .15 censuses + .16 band/floor | Automated |
| **G2** | Anti-vacuous: every census gate proves non-empty domain (pier ≥ 2,276; fence > 30,000) | Structure |
| **G3** | Zero `PASS (waived)` without fresh dated operator quote | Waiver rule |
| **G4** | All T-152.12–.21 verify commands re-run green at tip | Regression |
| **G5** | O1–O12 all signed PASS by operator; ≥ 12 screenshots committed | Operator |
| **G6** | Any FAIL → corresponding T-152.2x hotfix slice exists in registry before close | Discipline |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
# extended master (exact entry per .10's runner + additions)
make ci-local || true   # platform replay
cargo test -p map-engine-core && cargo test -p map-engine-render && make wasm
scripts/website/wgpu-gpu-verify.sh || make wgpu-verify     # GPU text + readback gates
cd apps/website/frontend && npm test && npm run build && npm run lint
ls .ai/artifacts/t152_22_operator/ | wc -l   # ≥ 12 after operator pass
```

---

## Manual acceptance

- **M1 (operator):** O1–O12 browser pass + signatures + screenshots.
- **M2 (operator):** merge/`ticket done` go/no-go decision recorded.

---

## Documentation sync (Cursor, after sign-off)

Registry `T-152.22 → shipped`; program status → complete-pending-merge; hub closure notes; `./scripts/ticket sync`. Do **not** mark T-152 `done` before merge decision.

---

## Claude Code prompt — T-152.22 (copy-paste)

Authority: this spec. **Operator drives; agent assists automated half only. Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Assist **T-152.22** — E2E re-gate + operator O1–O12 (executor: human).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git tag -l 'T-152.1*' 'T-152.2*'   # expect .12–.21 all tagged
  make wasm

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_22_e2e_regate_operator.md
  2. docs/specs/Mission_Creator_Architecture/t152_10_e2e_cartographic_gate.md (O1–O12 source)
  3. .ai/artifacts/t152_11_fidelity_audit_report.md §6.5 (vacuity holes to close)
  4. .ai/artifacts/t152_12..21_verify_log.md (all)

═══ PROBLEM ═══
  .10 passed on vacuous/waived/CPU-only gates with every operator row PENDING. Re-gate with
  structural anti-vacuity + GPU gates, then the human signs O1–O12 with screenshots.

═══ SHIPPED (do not reopen) ═══
  .12–.21 remediations — FAILs become filed T-152.2x hotfix slices, not inline patches.

═══ LANGUAGE GATE ═══
  Gate scripts/tests only; no feature code.

═══ LOCKED ═══
  - Census gates assert non-empty domains (pier ≥ 2,276; fence > 30,000)
  - PASS (waived) needs fresh dated operator quote
  - O-rows signed by human; ≥12 screenshots in .ai/artifacts/t152_22_operator/
  - Any FAIL ⇒ hotfix slice filed before close

═══ DO ═══
  1. Extend + run the automated master (G1–G4)
  2. Prep O1–O12 evidence sheet for operator
  3. Record signatures + screenshots; file hotfix slices for FAILs
  4. .ai/artifacts/t152_22_verify_log.md; commit "T-152.22: ..."; tag T-152.22

═══ DO NOT ═══
  - Sign any O-row yourself
  - Patch failures inline
  - Merge to main / mark ticket done

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1 O1–O12 sign-off · M2 merge decision

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - O1–O12 result table + screenshot count
  - List of filed hotfix slices (if any)
```
