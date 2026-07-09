# T-151.10 — Claude Code handoff (Fable 5 program audit)

**Spec (wins on conflict):**
[`t151_10_fable_program_audit.md`](../../docs/specs/Mission_Creator_Architecture/t151_10_fable_program_audit.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
§W10 · **Working tree:** `tbd-reforger-wgpu-spike/` @ **T-151.9** — **never `main`**.

## LANGUAGE GATE (D5)

This slice is **audit-only**. Do not implement features. Do not grow TS engine policy.
Document D5 leaks as **OPEN** findings.

## Operator note

- Prefer **Fable 5** at **max / UltraCode** effort — full-program audit, not a skim.
- **T-069** is parked (`queued`) until this audit closes and Cursor tickets remediations.
- Format reference: [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md).

## CURRENT STATE

| Piece | Status |
|-------|--------|
| T-151 W0–W9 | Shipped; Deck retired @ `c4831451` |
| T-151.10 | **ready** — this handoff |
| T-069 markers | `queued` (blocked on audit + remediations) |

## What you are building

1. Living audit report: `.ai/artifacts/t151_10_fable_audit_report.md`
2. Verify log with Class R/S sample evidence: `.ai/artifacts/t151_10_verify_log.md`
3. Tag **T-151.10**

## Do not

- Edit docs/registry/CLAUDE/ROADMAP (Cursor).
- Ship T-069 or any feature code under this tag.
- Soft-pass unverified claims.

## Return

SHA + tag · report path · verify log · list of OPEN findings for Cursor remediation tickets.
