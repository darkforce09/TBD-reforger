# Send-off — T-130 (Fable remainder) ← **parallel worktree**

**Checkout:** `.ai/artifacts/worktrees/TBD-T-130` · **Branch:** `ticket/T-130`

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-130
git merge main
./scripts/ticket brief T-130
./scripts/ticket prompt T-130
```

| Doc | Path |
|-----|------|
| Handoff | [`.ai/artifacts/t130_claude_code_handoff.md`](t130_claude_code_handoff.md) |
| Spec | [`docs/platform/t130_fable_audit_remainder.md`](../../docs/platform/t130_fable_audit_remainder.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |
| Operator | [`.ai/artifacts/t130_audit_operator_resume.md`](t130_audit_operator_resume.md) |

**Do not block:** T-090.1.2.8 on **main** runs in a separate Claude session.

---

## Copy-paste prompt — Batch 1 (T-130.1 → T-130.3)

```
Read CLAUDE.md first.

Implement **T-130** slices **T-130.1, T-130.2, T-130.3** — Fable audit remainder (backend + Discord + CI).

═══ PREFLIGHT ═══
  cd .ai/artifacts/worktrees/TBD-T-130
  git merge main
  ./scripts/ticket brief T-130

═══ READ ═══
  1. .ai/artifacts/t130_claude_code_handoff.md
  2. docs/platform/t130_fable_audit_remainder.md
  3. .ai/artifacts/fable_5_omni_audit_report.md  (OPEN rows F2B-*, F3-*)
  4. apps/website/internal/handlers/missions.go
  5. apps/website/internal/handlers/auth.go
  6. apps/website/internal/middleware/ratelimit.go
  7. apps/website/internal/services/discord.go + webhook.go
  8. .github/workflows/ci.yml + Makefile (ci-local)

═══ PROBLEM ═══
  Fable audit OPEN: list count swallowed, silent empty export, refresh rows never purged,
  ratelimit prefix footgun, no Discord 429 handling, webhook title uncapped, CI skips services tests.

═══ LOCKED ═══
  - Work on branch ticket/T-130 only
  - No docs/registry/CLAUDE marker edits
  - Do not touch T-090 satellite / basemap code

═══ DO ═══
  1. T-130.1 — F2B-07, 08, 09, 11 + tests
  2. T-130.2 — F3-01, 02, 03 + service tests
  3. T-130.3 — F2B-06 CI + make ci-local
  4. .ai/artifacts/t130_verify_log.md (batch 1 section)
  5. Commit prefix T-130.1: / T-130.2: / T-130.3: on ticket/T-130

═══ VERIFY ═══
  go test ./internal/services/... ./internal/middleware/... ./internal/handlers/...
  cd apps/website/frontend && npm run build && npm run lint

═══ RETURN ═══
  SHA(s) on ticket/T-130 · verify log · Ready for Cursor doc sync after full T-130 ships.
```

---

## Copy-paste prompt — Batch 2 (T-130.4 → T-130.6)

```
Read CLAUDE.md first.

Continue **T-130** on ticket/T-130 — slices **T-130.4, T-130.5, T-130.6** (mod + MC + mission lifecycle).

═══ PREFLIGHT ═══
  cd .ai/artifacts/worktrees/TBD-T-130
  git pull   # or merge main if needed
  ./scripts/ticket brief T-130

═══ READ ═══
  1. .ai/artifacts/t130_claude_code_handoff.md  (§ T-130.4–.6)
  2. docs/platform/t130_fable_audit_remainder.md
  3. apps/mod/tbd-framework/Scripts/Game/TBD/Backend/*.c
  4. apps/mod/tbd-framework/Scripts/Game/TBD/Export/*.c
  5. features/mission-creator/hooks/useMissionEditor.ts
  6. features/mission-creator/MissionCreatorPage.tsx
  7. pages/admin.tsx
  8. internal/handlers/cms.go  (archive pattern)
  9. internal/models/mission.go

═══ DO ═══
  1. T-130.4 — mod F1-16…20
  2. T-130.5 — F4-03 new-tab conflict, F4-07 non-UUID, F2F-07 admin Dialog
  3. T-130.6 — mission archive/delete API + library UI
  4. Append t130_verify_log.md batch 2
  5. Tag **T-130** when all slices done · Ready for merge to main

═══ VERIFY ═══
  cd apps/website/frontend && npm run build && npm run lint
  make test-it  (if db-up)

═══ RETURN ═══
  Full verify log · merge ticket/T-130 → main instructions for operator.
```

**After ship:** tell Cursor **"doc sync for T-130"** → T-130.7 (Cursor) + living tracker zero OPEN.
