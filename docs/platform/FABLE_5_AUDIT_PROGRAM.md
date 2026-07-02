# Fable 5 audit remediation program — COMPLETE

**Program complete (2026-07-02)** — T-126 ✓ → T-127 ✓ → T-128 ✓. **Remainder:** **T-130** (parallel worktree) drains OPEN/PARTIAL. **Main queue:** **T-090.1.2.8** / [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md).

**Source:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) — **living tracker** (remediation index + inline status; remaining OPEN/DEFERRED findings tracked there)

**Execution order (as run):**

```text
1. T-126  Security + auth follow-up          (claude-code)  ✓ shipped @ 4a47688e
2. T-127  Mission Creator UX fixes           (claude-code)  ✓ shipped @ 0515aabb
3. T-128  Doc link repair + staging honesty  (cursor-docs)  ✓ shipped (tag T-128)
— RESUMED: T-090.1.2.8 (satellite) / T-068 / TICKET_LEAD
```

**Unpaused (program done):**

| Ticket | What |
|--------|------|
| **T-090.1.2.8** | Unified GPU satellite texture — **shipped** @ `db9057ef` |
| **T-090.1.2.5** | Satellite water composite — **next on main** |
| **T-068** Phase 2 | Virtual Arsenal (still gated on map/ORBAT, per its own program) |
| Everything else | See [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) |
| **OPEN + PARTIAL (~22 findings)** | **T-130** — [`t130_fable_audit_remainder.md`](t130_fable_audit_remainder.md) · worktree `ticket/T-130` |

**Not in Fable program (later):**

| Finding | Ticket |
|---------|--------|
| Mod `/compiled` + roster REST | **T-092** |
| Map tile pyramid | **T-090.1.1** |
| Mission archive/delete | Future |
| Discord 429 | Deferred |

**Floor selector** id → **T-129** (was T-126).

---

## Operator quick start (post-program)

```bash
git pull && git lfs pull
# Merge order: T-127 worktree → main, then T-128 worktree → main (resolve registry.json
# by keeping T-127's row from main + T-128/T-090 rows from ticket/T-128), then:
./scripts/ticket sync && ./scripts/ticket check
./scripts/ticket prompt T-090    # next: T-090.1.2.8 unified satellite texture
```

**T-126** shipped @ `4a47688e` (tag **T-126**).  
**T-127** shipped (MC UX U1–U4 + U5 stretch, `ticket/T-127` worktree).  
**T-128** shipped (tag **T-128**, `ticket/T-128` worktree) — log [`t128_doc_link_repair_log.md`](../../.ai/artifacts/t128_doc_link_repair_log.md)

One-pager: [`.ai/artifacts/fable_audit_operator_resume.md`](../../.ai/artifacts/fable_audit_operator_resume.md)

---

## Ticket index

| ID | Status | Spec | Executor |
|----|--------|------|----------|
| **T-126** | shipped @ `4a47688e` | [`t126_audit_security_followup.md`](t126_audit_security_followup.md) | claude-code |
| **T-127** | shipped @ `0515aabb` | [`t127_mc_ux_audit_fixes.md`](t127_mc_ux_audit_fixes.md) | claude-code |
| **T-128** | shipped (tag `T-128`) | [`t128_doc_link_repair.md`](t128_doc_link_repair.md) | cursor-docs |
