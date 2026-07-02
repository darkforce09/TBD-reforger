# Fable audit — operator resume (PROGRAM COMPLETE)

**T-126 ✓ → T-127 ✓ → T-128 ✓ (2026-07-02).** Normal queue resumes at **T-090.1.2.8**.

| Doc | Purpose |
|-----|---------|
| Hub | [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |
| T-128 log | [`.ai/artifacts/t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md) |

---

## Done: T-126 @ `4a47688e` · T-127 @ `0515aabb` · T-128 (tag **T-128**)

Verify: [t126_verify_log.md](t126_verify_log.md) · [t127_verify_log.md](t127_verify_log.md) · [t128_doc_link_repair_log.md](t128_doc_link_repair_log.md)

**T-127** — U1–U5 MC UX (conflict IDB persist, export toasts, basemap coerce, folder delete confirm, ORBAT 409 messages).

**T-128** — Handoff link depths, staging T-092 honesty, README rewrites, orphan `frontend/docs/` deleted, floor picker → T-129, living tracker updated.

---

## Post-merge operator checklist

```bash
./scripts/ticket sync && ./scripts/ticket check && make ticket-check-strict
rmdir apps/website/internal/handlers/missions 2>/dev/null || true
./scripts/ticket prompt T-090    # next: T-090.1.2.8 unified satellite texture
./scripts/ticket clean T-127
./scripts/ticket clean T-128
```

---

## Resumed

T-090.1.2.8 (**next**) · T-068 Phase 2 (still map/ORBAT-gated per its program) · map / spawn / arsenal queues per [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md)
