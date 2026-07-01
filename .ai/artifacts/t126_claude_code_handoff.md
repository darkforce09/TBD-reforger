# T-126 — Claude Code handoff (audit security)

**Slice:** T-126 · **Executor:** claude-code · **Branch:** commit to `main`  
**Spec:** [`docs/platform/t126_audit_security_followup.md`](../../docs/platform/t126_audit_security_followup.md)  
**Audit:** [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md)

---

## Execution order

**T-126 first** in Fable program — then T-127, then T-128 (Cursor docs).

---

## S1 — ExportMission leak (HIGH)

```go
// missions.go ExportMission — add before buildMissionDoc:
if !h.canViewMission(c, m) {
    c.JSON(http.StatusNotFound, gin.H{"error": "mission not found"})
    return
}
```

Test: two mission_makers, draft export cross-author → 404.

---

## S2 — Refresh rotation (HIGH)

Replace check-then-update with:

```sql
UPDATE refresh_tokens SET revoked_at = $now WHERE id = $id AND revoked_at IS NULL
```

`RowsAffected != 1` → treat as reuse (revoke family / 401). See OWASP refresh best practice.

---

## S3 — Slot claim race (MED)

`events.go` RegisterForEventMission ~748: use conditional UPDATE on `assigned_to IS NULL OR assigned_to = me`. Count `RowsAffected`. Re-run capacity check inside same transaction with row lock.

---

## S4–S6 — Frontend auth

| File | Fix |
|------|-----|
| `client.ts:44-46` | Persist `refresh_token` on 401-retry success always |
| `useAuthBootstrap.ts:43-45` | Split catch: rotation fail → clearSession; /me fail → keep tokens |
| `pages/auth.tsx` | Same pattern on OAuth callback if applicable |

---

## Return

SHA + tag **T-126** · `.ai/artifacts/t126_verify_log.md` · **Ready for Cursor doc sync.**
