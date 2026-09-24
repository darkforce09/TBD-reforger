# Fable 5 audit remediation program — COMPLETE

**Program complete (2026-07-03)** — T-126 ✓ → T-127 ✓ → T-128 ✓ → **T-130 ✓** @ `90c9f261` (tag **T-130**). **Main queue:** **T-090.1.2.5** / [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md).

**Source:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) — **living tracker** (OPEN: **F5-10** spelling only — deferred trivial)

**Execution order (as run):**

```text
1. T-126  Security + auth follow-up          (claude-code)  ✓ shipped @ 4a47688e
2. T-127  Mission Creator UX fixes           (claude-code)  ✓ shipped @ 0515aabb
3. T-128  Doc link repair + staging honesty  (cursor-docs)  ✓ shipped (tag T-128)
4. T-130  OPEN + PARTIAL remainder           (claude-code + cursor-docs) ✓ @ 90c9f261
— RESUMED: T-090.1.2.5 (water) / T-068 / TICKET_LEAD
```

**Unpaused (Fable closed):**

| Ticket | What |
|--------|------|
| **T-090.1.2.8** | Unified GPU satellite texture — **shipped** @ `db9057ef` |
| **T-090.1.2.5** | Satellite water composite — **active on main** |
| **T-068** Phase 2 | Virtual Arsenal (still gated on map/ORBAT, per its own program) |
| Everything else | See [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) |

**Not in Fable program (later):**

| Finding | Ticket |
|---------|--------|
| Mod `/compiled` + roster REST | **T-092** |
| Map tile pyramid | **T-090.1.1** |
| Spelling dialect | **F5-10** deferred |

**Floor selector** id → **T-129** (was T-126).

**North Star gaps (unplanned backlog):** [`tbd_north_star_backlog.md`](tbd_north_star_backlog.md) · registry **T-131…T-142** (`idea`).

---

## Operator quick start (post-program)

```bash
git pull && git lfs pull
./scripts/ticket sync && ./scripts/ticket check
./scripts/ticket prompt T-090    # next: T-090.1.2.5 satellite water
```

**T-126** shipped @ `4a47688e` (tag **T-126**).  
**T-127** shipped @ `0515aabb`.  
**T-128** shipped (tag **T-128**).  
**T-130** shipped @ `90c9f261` (tag **T-130**) — verify [`.ai/artifacts/t130_verify_log.md`](../../.ai/artifacts/t130_verify_log.md)

One-pager: [`.ai/artifacts/fable_audit_operator_resume.md`](../../.ai/artifacts/fable_audit_operator_resume.md)

---

## Ticket index

| ID | Status | Spec | Executor |
|----|--------|------|----------|
| **T-126** | shipped @ `4a47688e` | [`t126_audit_security_followup.md`](t126_audit_security_followup.md) | claude-code |
| **T-127** | shipped @ `0515aabb` | [`t127_mc_ux_audit_fixes.md`](t127_mc_ux_audit_fixes.md) | claude-code |
| **T-128** | shipped (tag `T-128`) | [`t128_doc_link_repair.md`](t128_doc_link_repair.md) | cursor-docs |
| **T-130** | shipped @ `90c9f261` (tag `T-130`) | [`t130_fable_audit_remainder.md`](t130_fable_audit_remainder.md) | claude-code + cursor-docs |
