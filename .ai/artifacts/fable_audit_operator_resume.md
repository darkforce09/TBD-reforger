# Fable audit — operator resume (PROGRAM COMPLETE)

**T-126 ✓ → T-127 ✓ → T-128 ✓ (2026-07-02).** Normal queue resumes at **T-090.1.2.8**.

| Doc | Purpose |
|-----|---------|
| Hub | [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |
| T-128 log | [`.ai/artifacts/t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md) |

---

## Done: T-126 @ `4a47688e` (tag **T-126**)

Security S1–S6 · verify [`.ai/artifacts/t126_verify_log.md`](t126_verify_log.md)

## Done: T-127 (MC UX, `ticket/T-127` worktree)

U1–U4 (+ U5 stretch) · verify `t127_verify_log.md` · tracker rows flip on its merge

## Done: T-128 (docs, `ticket/T-128` worktree, tag **T-128**)

Handoff link depths, staging T-092 honesty, README rewrites, orphan tree deleted, floor picker → T-129, living tracker updated. Log: [`t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md)

---

## Merge order (operator)

1. Merge `ticket/T-127` → `main`.
2. Merge `ticket/T-128` → `main` — `registry.json` conflict expected: keep T-127's row from main + T-128/T-090 rows from this branch.
3. On main: `./scripts/ticket sync && ./scripts/ticket check` · `rmdir apps/website/internal/handlers/missions` (untracked empty dir).
4. `./scripts/ticket prompt T-090` → **T-090.1.2.8** unified satellite texture.

---

## Resumed

T-090.1.2.8 (**next**) · T-068 Phase 2 (still map/ORBAT-gated per its program) · map / spawn / arsenal queues per [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md)
