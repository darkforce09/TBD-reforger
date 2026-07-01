# T-126 — Fable audit security + auth follow-up

**Ticket:** T-126 · **Executor:** claude-code · **Status:** **SHIPPED** @ `4a47688e` (tag **T-126**)  
**Verify:** [`.ai/artifacts/t126_verify_log.md`](../../.ai/artifacts/t126_verify_log.md)  
**Authority:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) §2 Backend · [`FABLE_5_AUDIT_PROGRAM.md`](FABLE_5_AUDIT_PROGRAM.md)  
**Parent:** T-122 @ `f131770` (Export visibility missed T-122 T2 scope)

---

## In one sentence

Close **security and auth integrity** gaps from the Fable 5 audit — draft export leak, refresh-token rotation race, ORBAT slot claim race, and frontend refresh persistence bugs.

---

## Problem

Fable 5 re-verified the codebase and found **production-risk defects** T-122 did not fully close:

| ID | Finding | Severity |
|----|---------|----------|
| S1 | `ExportMission` skips `canViewMission` — any mission_maker exports another author's draft | HIGH |
| S2 | `Refresh` non-atomic revoke + no reuse detection — concurrent refresh forks token family | HIGH |
| S3 | ORBAT slot claim check-then-set — two users can claim same slot | MED |
| S4 | `Refresh` never checks `user.IsBanned` (belt-and-braces) | MED |
| S5 | `client.ts` 401-retry: no `user` in store → only `setAccessToken`, drops rotated refresh | MED |
| S6 | `useAuthBootstrap` + auth callback: `/me` fail after rotation → `clearSession()` kills valid refresh | MED |

---

## Goal

1. **S1** — `ExportMission` returns 404 when `!canViewMission` (same as GetMission).
2. **S2** — Atomic rotation: `UPDATE refresh_tokens SET revoked_at=now WHERE id=? AND revoked_at IS NULL`; require `RowsAffected==1`; on 0 rows treat as **reuse attack** → revoke token family / return 401.
3. **S3** — Slot assign: conditional `UPDATE orbat_slots SET assigned_to=?, assigned_at=now WHERE id=? AND event_mission_id=? AND (assigned_to IS NULL OR assigned_to=?)`; require `RowsAffected==1` or return 409 slot taken. Re-check capacity inside transaction with `SELECT FOR UPDATE` or equivalent count under lock.
4. **S4** — `Refresh` rejects banned users with 403.
5. **S5** — On successful refresh in axios interceptor: always persist **both** tokens via `setSession` or a new `setTokens({ access, refresh, expires_at })` even when `user` is absent; fetch `/me` on next navigation if needed.
6. **S6** — Bootstrap/callback: if rotation succeeds but `/me` fails, **retain rotated tokens** (do not `clearSession`); surface degraded state or retry — never revoke the only valid refresh on transient network error.

---

## Out of scope

- Mod REST `/compiled` / roster routes — **T-092**
- Mission archive/delete — future ticket
- Discord 429 — deferred
- CI expanding to `internal/services` — optional stretch if time remains

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Export deny | 404 not 403 (match GetMission T-122) |
| Rotation | Single-use preserved; reuse → invalidate family |
| Slot race | 409 with distinct error string (preserve existing messages where possible) |
| Frontend | Never drop rotated refresh_token on 401-retry or post-rotation /me blip |
| Tests | Add/adapt integration tests in `missions_integration_test.go` / `events_integration_test.go` / auth tests |
| Docs/registry | Claude does **not** edit — Cursor after ship |

---

## Tasks

1. `internal/handlers/missions.go` — `ExportMission`: add `canViewMission` gate before `buildMissionDoc`.
2. `internal/handlers/auth.go` — atomic refresh rotation + reuse handling + `IsBanned` check.
3. `internal/handlers/events.go` — conditional slot UPDATE + locked capacity check in register transaction.
4. `apps/website/frontend/src/api/client.ts` — persist full token pair on 401-retry success.
5. `apps/website/frontend/src/hooks/useAuthBootstrap.ts` — distinguish rotation success vs /me failure.
6. `apps/website/frontend/src/pages/auth.tsx` (callback) — same pattern as bootstrap if applicable.
7. Integration tests proving S1–S3 (and S2 reuse if feasible).

---

## Verify

```bash
make test-it
cd apps/website/frontend && npm run build && npm run lint
```

**Manual S1:** dev-login as mission_maker A, create draft; dev-login as mission_maker B, `GET /api/v1/missions/:id/export` → 404.  
**Manual S2:** document concurrent refresh test approach in verify log.  
**Manual S3:** two-browser slot claim race → one 409.

---

## Documentation sync (Cursor — after ship)

Registry `shipped_at` · `docs/platform/CODEBASE_AUDIT_2026.md` add Fable S1–S6 section · `./scripts/ticket sync`.

---

## Claude Code prompt — T-126 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**  
Extract: `./scripts/ticket prompt T-126` · standard: [`.ai/tickets/CLAUDE_CODE_PROMPT.md`](../../.ai/tickets/CLAUDE_CODE_PROMPT.md)

```
Read CLAUDE.md first.

Implement **T-126** — Fable audit security + auth follow-up (S1–S6).

═══ PREFLIGHT ═══
  git pull && git lfs pull
  ./scripts/ticket brief T-126

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t126_claude_code_handoff.md
  2. docs/platform/t126_audit_security_followup.md
  3. .ai/artifacts/fable_5_omni_audit_report.md  (§2 Backend — cite line refs)
  4. internal/handlers/missions.go  (canViewMission pattern @ GetMission)
  5. internal/handlers/auth.go
  6. internal/handlers/events.go  (RegisterForEventMission slot branch)
  7. apps/website/frontend/src/api/client.ts
  8. apps/website/frontend/src/hooks/useAuthBootstrap.ts

═══ PROBLEM ═══
  Fable 5 audit found draft mission export leak (ExportMission missing canViewMission),
  non-atomic refresh rotation, ORBAT slot claim race, and frontend paths that drop the
  single-use refresh token after rotation — causing random logouts.

═══ SHIPPED (do not reopen) ═══
  - T-122 @ f131770 — GetMission/GetVersion canViewMission
  - T-123 auth contract tags

═══ LOCKED ═══
  - S1–S6 all required; see spec §Goal table
  - Export deny = 404; rotation reuse → 401 + family revoke
  - Slot race = conditional UPDATE + RowsAffected
  - Frontend: never clearSession after successful rotation on /me transient fail
  - No docs/registry edits

═══ DO ═══
  1. S1 — ExportMission canViewMission gate + integration test
  2. S2 — atomic refresh UPDATE + reuse detection + test if feasible
  3. S3 — slot conditional assign + capacity under transaction lock + test
  4. S4 — Refresh IsBanned check
  5. S5 — client.ts full token pair on 401-retry
  6. S6 — useAuthBootstrap + auth callback rotation/me split
  7. .ai/artifacts/t126_verify_log.md — S1–S6 + test output
  8. Tag **T-126** · prefix **T-126:**

═══ DO NOT ═══
  - Edit docs/**, registry, TICKET_*.md, CLAUDE markers
  - Mod REST routes (T-092)
  - Weaken T-122 visibility on GET routes

═══ VERIFY (all exit 0) ═══
  make test-it
  cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  S1: cross-author draft export → 404
  S5/S6: boot after rotation — session survives /me blip (note in log)

═══ RETURN ═══
  - Commit SHA + tag T-126
  - t126_verify_log.md + tests added
  - **Ready for Cursor doc sync.**
```
