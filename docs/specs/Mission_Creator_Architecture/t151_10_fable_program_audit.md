# T-151.10 — Fable 5 full-program audit (W10)

**Status:** **shipped** @ `6adbd4bf` (tag **T-151.10**) · round-2 **T-151.10.1** @ `40def01a` ·
remediations **T-151.11.1–.6** · **Program:** T-151 W10 · **Executor:** claude-code (Fable 5) ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** tag **T-151.9** ship `c4831451` / tip `58c8fcc3`+ ·
verify [`.ai/artifacts/t151_9_verify_log.md`](../../../.ai/artifacts/t151_9_verify_log.md) ·
**Tracker:** [`.ai/artifacts/t151_10_fable_audit_report.md`](../../../.ai/artifacts/t151_10_fable_audit_report.md).

## In one sentence

Fable 5 performs a **complete, adversarial audit** of everything shipped under **T-151**
(W0–W9): architecture, LANGUAGE GATE (D5), Class R/S gates, Deck retirement honesty, wasm
surface, and residual risk — and writes a living tracker report before any W10 feature work
(T-069+).

## Problem

W0–W9 flipped Mission Creator to wgpu and deleted the Deck runtime. That is a large surface
(crates + FE bridge + oracles + goldens + verify logs). Before markers/vehicles (T-069/T-070),
we need an independent Fable 5 pass that **does not trust** prior verify logs — re-check
claims, find silent deferrals, LOC/policy leaks into TS, broken gates, and production-risk
defects.

## Goal

1. **Audit only (this slice):** produce
   [`.ai/artifacts/t151_10_fable_audit_report.md`](../../../.ai/artifacts/t151_10_fable_audit_report.md)
   — living tracker in the style of
   [`.ai/artifacts/fable_5_omni_audit_report.md`](../../../.ai/artifacts/fable_5_omni_audit_report.md).
2. Cover **every** T-151.x slice (0…9 + hotfixes 4.1 / 5.1 / 7.1–7.3 / 8.1) against hub
   locked decisions **D1–D5** and LANGUAGE GATE D5.
3. Classify each finding: **PASS / PARTIAL / OPEN** + severity **S/R/T/M/D** (security /
   reliability / tech-debt / maintainability / docs).
4. Re-run or cite **Class R** (oracle/golden) and **Class S** (ship) evidence; call out any
   verify-log claim that is not reproducible.
5. **No feature work** (no T-069 markers). **No silent fixes** — if a one-line honesty fix
   in a verify log is needed, do it; app-code remediation is **out of scope** unless the
   operator later opens a follow-up slice from the report.
6. Tag **T-151.10** on the audit-report commit (docs/artifact only is fine). Cursor doc-sync
   after return.

## Out of scope

- Implementing T-069 / T-070 / ruler / LoS.
- Editing `.ai/tickets/registry.json`, hub prose beyond what Cursor owns, or CLAUDE.md
  (Claude: report + optional verify-log honesty only).
- Reintroducing Deck or growing fat TS controllers.
- Expanding scope to T-090 / T-145 / whole-platform Fable (T-126 program already closed).

## Locked decisions

| ID | Choice |
|----|--------|
| W10 meaning | **T-151.10 = program audit**, not markers. Markers stay **T-069** (queued until audit closes). |
| Deliverable | Living markdown tracker under `.ai/artifacts/t151_10_fable_audit_report.md` |
| Code changes | **None** in this slice (except verify-log typo/honesty if found). Fixes → follow-up slices. |
| Model | Fable 5 at **high** (max OK) — **never UltraCode** |
| Agent budget | ≤ **2** concurrent subagents total; default sequential main-thread |
| Worktree | Spike only; never `main` |
| Authority | Hub + each slice spec + verify logs + live tree; **code wins** over stale docs |

## Audit checklist (must cover)

### A — Program integrity
- Hub **D1–D5** still true in code (always wgpu; no Deck escape hatch; flip+delete same tag era).
- `MissionCreatorPage` mounts only `WgpuTacticalMap`; no Deck import path in app runtime.
- `package.json`: deck/luma only in `devDependencies`; production `dist` Deck/luma-free.
- Vitest count / oracle story matches T-151.9 claims (`N = 281` baseline unless tree moved).

### B — LANGUAGE GATE (D5)
- Inventory TS under `tactical-map/wgpu/` + related: LOC budgets (`wgpuSlots.ts` ≤ 60).
- No engine policy / LOD / residency / camera math / pack helpers living in `.ts` that belong
  in `crates/map-engine-*`.
- No silent “finish in TS” leftovers from W6–W9.

### C — Slice-by-slice (W0–W9)
For each shipped slice: read spec + verify log + spot-check key files; mark PASS/PARTIAL/OPEN.
Call out deferred items that were never named in a verify log (**silent deferral** = OPEN).

### D — Correctness / Class R
- World parity / pick / residency oracles + goldens still wired and green.
- Compute cull Class R (if claimed) still present.
- Any hybrid Deck oracle residue that should have been deleted.

### E — Reliability / security (map surface)
- Wasm init failure modes; asset path assumptions; no accidental service of secrets.
- Worker / Comlink leftovers that should be gone post-Deck.

### F — Docs honesty
- CLAUDE / hub / verify logs vs git tags; ship SHA vs tag tip mismatches called out (not
  necessarily bugs).

## Tasks

1. Read hub + all T-151.x specs + verify logs listed in CLAUDE §T-151 Done.
2. Walk `crates/map-engine-*`, `apps/website/frontend/src/features/tactical-map/wgpu/`,
   `_wasm/oracles/`, goldens, `MissionCreatorPage` mount path.
3. Re-run or sample Class R/S commands from T-151.9 verify log; record PASS/FAIL with SHA.
4. Write `.ai/artifacts/t151_10_fable_audit_report.md` with index table + per-finding detail.
5. Write `.ai/artifacts/t151_10_verify_log.md` (audit completion evidence + command outputs).
6. Commit + tag **T-151.10**. Return to Cursor for registry sync + remediation tickets.

## Verify

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
# Sample Class S (cite full output in verify log; do not skip on red)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
# Spot: dist Deck-free + wgpuSlots LOC
rg -n "deck\\.gl|@deck\\.gl|@luma\\.gl" dist/assets || true
wc -l src/features/tactical-map/wgpu/wgpuSlots.ts
```

## Acceptance

- [ ] Report exists with findings indexed (PASS/PARTIAL/OPEN + severity).
- [ ] Every W0–W9 slice has at least one row in the index.
- [ ] LANGUAGE GATE section with concrete file:line citations for any leak.
- [ ] Class R/S sample results recorded in verify log.
- [ ] No T-069 / feature code shipped under this tag.
- [ ] Tag **T-151.10** on the audit commit.

## Claude Code prompt — T-151.10 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry/CLAUDE** (report + verify log OK).

```
Read CLAUDE.md first. Work ONLY in tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.10** — Fable 5 full-program audit of T-151 (W0–W9).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git rev-parse HEAD
  git lfs pull && make map-assets-link
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_10_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_10_fable_program_audit.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md
  4. .ai/artifacts/fable_5_omni_audit_report.md  (format reference)
  5. .ai/tickets/CLAUDE_CODE_PROMPT.md  (§T-151 language gate)
  6. .cursor/rules/no-silent-deferrals.mdc
  7. All T-151.x verify logs under .ai/artifacts/t151_*_verify_log.md

═══ PROBLEM ═══
  W0–W9 shipped and Deck is retired, but no independent Fable 5 audit has re-verified
  claims, LANGUAGE GATE, Class R/S, and silent deferrals before W10 features (T-069+).

═══ SHIPPED (do not reopen as features) ═══
  T-151.0 … T-151.9 (+ hotfixes). Audit them; do not re-implement.

═══ LANGUAGE GATE (T-151 — MANDATORY) ═══
  This slice is AUDIT-ONLY. Do not add engine policy in TS. Do not grow wgpuSlots.ts.
  If you find a D5 leak, document it as OPEN — do not "just fix in TS".

═══ DO ═══
  - Adversarial audit of crates + FE wgpu path + oracles + goldens + package/dist claims
  - Write .ai/artifacts/t151_10_fable_audit_report.md (living tracker)
  - Write .ai/artifacts/t151_10_verify_log.md with command evidence
  - Commit + tag T-151.10

═══ DO NOT ═══
  - Edit registry / hub / CLAUDE / ROADMAP (Cursor owns)
  - Implement T-069 markers or any W10 feature
  - Silently fix app code (report findings; verify-log honesty OK)
  - Soft-language the report — Class S/R claims must be PASS or FAIL with evidence

═══ VERIFY ═══
  (commands in spec §Verify — paste outputs into verify log)

═══ RETURN ═══
  SHA + tag T-151.10
  Paths to audit report + verify log
  Ready for Cursor: registry sync + remediation tickets from OPEN findings
```
