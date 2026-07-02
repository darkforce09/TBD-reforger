# Fable audit — operator resume (ONLY active work)

| Doc | Purpose |
|-----|---------|
| Hub | [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../docs/platform/FABLE_5_AUDIT_PROGRAM.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |

---

## Done: T-126 @ `4a47688e` · T-127 @ `0515aabb`

Verify: [t126_verify_log.md](t126_verify_log.md) · [t127_verify_log.md](t127_verify_log.md)

---

## Now: T-128 (docs) ← **YOU ARE HERE**

**Worktree:** `.ai/artifacts/worktrees/TBD-T-128` · branch `ticket/T-128`

Rebase onto latest `main` first (includes T-127 merge):

```bash
git -C .ai/artifacts/worktrees/TBD-T-128 merge main
```

Spec: [`docs/platform/t128_doc_link_repair.md`](../docs/platform/t128_doc_link_repair.md)

**Verify:** `./scripts/ticket sync && ./scripts/ticket check`  
**Ship:** tag **T-128** · `t128_doc_link_repair_log.md` · update living tracker

---

## Paused until T-128 ships

T-090.1.2.8 · T-068 Phase 2
