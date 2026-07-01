# T-126 — Verify log (Fable audit security + auth follow-up)

**Ticket:** T-126 · **Executor:** claude-code · **Tag:** T-126 · **Base:** `0d6fe485`
**Authority:** `docs/platform/t126_audit_security_followup.md` + `.ai/artifacts/t126_claude_code_handoff.md` · audit `.ai/artifacts/fable_5_omni_audit_report.md` §2 Backend

## Slice → change → proof

| ID | Fix | File(s) | Proof |
|----|-----|---------|-------|
| **S1** | `ExportMission` gates on `canViewMission` → 404 for cross-author drafts (audit missions.go:758) | `internal/handlers/missions.go` | `TestExportMissionVisibility` PASS |
| **S2** | Atomic rotation (`UPDATE … WHERE id=? AND revoked_at IS NULL`, `RowsAffected==1`) + reuse detection → 401 + **token-family revoke** | `internal/handlers/auth.go` | `TestRefreshReuseRevokesFamily` PASS |
| **S3** | Register tx: per-mission `FOR UPDATE` lock (capacity race) + conditional slot `UPDATE` w/ `RowsAffected` → 409 (double-claim race) | `internal/handlers/events.go` | `TestSlotClaimRace` PASS |
| **S4** | `Refresh` rejects banned users → 403 + family revoke | `internal/handlers/auth.go` | `TestRefreshBannedRejected` PASS |
| **S5** | 401-retry with no user in store persists the **full rotated pair** via new `setTokens` (was dropping the rotated refresh token) | `frontend/src/store/useAuthStore.ts`, `frontend/src/api/client.ts` | build + lint clean; see manual note |
| **S6** | Bootstrap + OAuth callback: rotation success followed by transient `/me` failure **retains** the rotated pair (no `clearSession`); only a failed rotation clears | `frontend/src/hooks/useAuthBootstrap.ts`, `frontend/src/pages/auth.tsx` | build + lint clean; see manual note |

## Automated verify (all exit 0)

**`make test-it`** (Go integration, real Postgres @ :5434):
```
ok  github.com/tbd-milsim/reforger-backend/internal/handlers  3.908s
```
Full package green — includes the pre-existing identity/mission/event flows (regression) plus the four new T-126 tests. Verbose run of the new tests:
```
--- PASS: TestRefreshReuseRevokesFamily (0.20s)
--- PASS: TestRefreshBannedRejected (0.20s)
--- PASS: TestSlotClaimRace (0.21s)
--- PASS: TestExportMissionVisibility (0.23s)
ok  github.com/tbd-milsim/reforger-backend/internal/handlers  0.855s
```

**Frontend** (`apps/website/frontend`, Node 26):
- `npm run build` → ✓ built (tsc + vite; only the pre-existing MissionCreatorPage chunk-size warning).
- `npm run lint` → clean (no eslint output).

`go vet ./internal/handlers/...` + `gofmt -l internal/` → clean.

## S2 concurrency note (test-if-feasible)

The deterministic proof is the sequential reuse test: rotate once (200) → replay the spent token (401, reuse) → the freshly issued token is then also 401 (family revoked), and a `COUNT(revoked_at IS NULL) == 0` assertion confirms the sweep. The concurrency the audit flagged (two simultaneous presentations of the same token) is closed by the atomic `UPDATE … WHERE revoked_at IS NULL` + `RowsAffected != 1 → reuse`: only one writer flips the row; the loser is treated as reuse and the family is revoked. `TestSlotClaimRace` exercises the analogous real-goroutine race for S3 (two concurrent claims → exactly one 200 / one 409) and passes.

## Manual verify

- **S1** — dev-login mission_maker A, create draft; dev-login mission_maker B, `GET /api/v1/missions/:id/export` → **404**. Covered end-to-end by `TestExportMissionVisibility` (create via API → cross-author export 404 → author export 200 → publish → cross-author export 200), so the manual browser step is redundant but was reasoned through against the route (`mm` group, `canViewMission`).
- **S5/S6** — reasoned trace (no live Discord in dev): with a persisted refresh token and `/me` returning a transient 5xx after a successful rotation, the store now retains the rotated pair (`setTokens`) instead of `clearSession()`, so `refreshToken` survives in `localStorage` (persisted via `partialize`) and a reload re-bootstraps + retries `/me`. Before T-126 the same blip dropped the single-use refresh token → forced re-login. The rotation-failure path still clears (correct: the stored token is dead).

## Scope

Touched exactly: `internal/handlers/{missions,auth,events}.go` + 3 integration test files (`missions_integration_test.go`, `events_integration_test.go`, new `auth_refresh_integration_test.go`) + `frontend/src/{store/useAuthStore.ts, api/client.ts, hooks/useAuthBootstrap.ts, pages/auth.tsx}` + this log. No docs/registry/CLAUDE-marker edits. Mod REST routes (T-092) and mission archive/delete untouched. T-122 GET visibility not weakened (asserted by the live-export leg of `TestExportMissionVisibility`).

**Ready for Cursor doc sync.**
