# Fable 5 audit remediation program

**ONLY ACTIVE WORK** — finish **T-126 → T-127 → T-128** before anything else (no parallel T-090 / map slices).

**Source:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) — **living tracker** (remediation index + inline status; update on each ship)

**Execution order (strict — one at a time):**

```text
1. T-126  Security + auth follow-up     (claude-code)  ✓ shipped @ 4a47688e
2. T-127  Mission Creator UX fixes      (claude-code)  ← YOU ARE HERE
3. T-128  Doc link repair + staging honesty (cursor-docs) — after T-127 ships
— then resume T-090.1.2.8 (satellite) / T-068 / TICKET_LEAD
```

**Paused until this program completes:**

| Ticket | What |
|--------|------|
| **T-090.1.2.8** | Unified GPU satellite texture |
| **T-068** Phase 2 | Virtual Arsenal |
| Everything else | See [`docs/TICKET_LEAD.md`](../docs/TICKET_LEAD.md) |

**Not in Fable program (later):**

| Finding | Ticket |
|---------|--------|
| Mod `/compiled` + roster REST | **T-092** |
| Map tile pyramid | **T-090.1.1** |
| Mission archive/delete | Future |
| Discord 429 | Deferred |

**Floor selector** id → **T-129** (was T-126).

---

## Operator quick start

```bash
git pull && git lfs pull
./scripts/ticket brief T-126
./scripts/ticket prompt T-126    # paste into Claude Code — ONLY this until T-126 ships
```

**T-126** shipped @ `4a47688e` (tag **T-126**) — doc sync done.  
After **T-127** ships → **"doc sync for T-127"** → Cursor ships **T-128**

One-pager: [`.ai/artifacts/fable_audit_operator_resume.md`](../../.ai/artifacts/fable_audit_operator_resume.md)

---

## Ticket index

| ID | Status | Spec | Executor |
|----|--------|------|----------|
| **T-126** | shipped @ `4a47688e` | [`t126_audit_security_followup.md`](t126_audit_security_followup.md) | claude-code |
| **T-127** | **active** | [`t127_mc_ux_audit_fixes.md`](t127_mc_ux_audit_fixes.md) | claude-code |
| **T-128** | queued | [`t128_doc_link_repair.md`](t128_doc_link_repair.md) | cursor-docs |
