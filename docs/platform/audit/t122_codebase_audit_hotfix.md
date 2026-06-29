# T-122 — Codebase audit hotfix (single bundle)

**Ticket:** T-122 · **Executor:** claude-code · **Status:** **shipped** @ `f131770` (tag **T-122**)  
**Authority:** [`CODEBASE_AUDIT_2026.md`](../CODEBASE_AUDIT_2026.md)

## In one sentence

Fix **all** audit findings (C, R, T, M, D) in **one branch** — 37/41 shipped; T1/T3/T8/T15 deferred with rationale in audit doc §Verification.

## Shipped scope

| Band | Result |
|------|--------|
| Critical C1–C4 | All shipped |
| Routing R1–R3 | All shipped |
| Tech debt | 13 shipped; T1/T3/T8 deferred |
| Minor M1–M15 | All shipped |
| Doc D1–D2 | D1 shipped; D2 stub + frontend doc link repair (Cursor doc pass @ merge) |

## Verify (replay)

```bash
make test-it
cd apps/website/frontend && npm run build && npm run lint
```

See [`CODEBASE_AUDIT_2026.md`](../CODEBASE_AUDIT_2026.md) §Verification for deferred rationale and mod Workbench note.
