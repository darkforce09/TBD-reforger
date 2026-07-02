# Fable audit — operator resume (ONLY active work)

**Do not start T-090.1.2.8, T-068, or anything else until T-126 → T-127 → T-128 are done.**

| Doc | Purpose |
|-----|---------|
| Hub | [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../docs/platform/FABLE_5_AUDIT_PROGRAM.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |
| Send-off | [`.ai/artifacts/t127_SEND_TO_CLAUDE.md`](t127_SEND_TO_CLAUDE.md) |

---

## Done: T-126 @ `4a47688e` (tag **T-126**)

Security S1–S6 · verify [`.ai/artifacts/t126_verify_log.md`](t126_verify_log.md)

---

## Now: T-127 (MC UX) ← **YOU ARE HERE**

```bash
git pull
./scripts/ticket brief T-127
./scripts/ticket prompt T-127
```

| ID | Fix | Tracker |
|----|-----|---------|
| **U1** | Conflict “Load server” → persist to IDB | F2F-03, F4-03 |
| **U2** | Export compile errors → toast | F2F-04, F4-05 |
| **U3** | `map` basemap → coerce satellite / degrade toast | F2F-05, F4-02 |
| **U4** | Folder delete → Aegis confirm | F4-01 |
| **U5** | *(stretch)* ORBAT 409 message mapping | F2F-06, F4-06 |

**Verify:** `cd apps/website/frontend && npm run build && npm run lint`  
**Ship:** tag **T-127** · `t127_verify_log.md` · **"doc sync for T-127"**

---

## Then: T-128 (Cursor)

**"ship T-128"** — handoff link depths, staging honesty, README rot, living tracker §5

---

## Paused

T-090.1.2.8 · T-068 Phase 2 · map / spawn / arsenal queues
