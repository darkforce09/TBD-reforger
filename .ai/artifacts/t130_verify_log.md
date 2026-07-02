# T-130 verify log

Branch `ticket/T-130` (worktree `.ai/artifacts/worktrees/TBD-T-130`). Sections per batch; Cursor flips tracker rows on doc sync.

---

## Batch 1 — T-130.1 / T-130.2 / T-130.3 (2026-07-02)

**Commits:**

| Slice | SHA | Findings |
|-------|-----|----------|
| T-130.1 | `6426600f` | F2B-07, F2B-08, F2B-09, F2B-11 |
| T-130.2 | `9db1b9e1` | F3-01, F3-02, F3-03 |
| T-130.3 | (this commit) | F2B-06 |

### Verification run

```
go build ./...                                              PASS
gofmt -l internal cmd                                       empty
golangci-lint run ./...                                     0 issues
go test ./internal/services/... -count=1                    ok (incl. 6 new: 429 retry ×2, embed caps, capRunes, empty client_id, authorize params)
go test ./internal/middleware/... -count=1                  ok (incl. TestRateLimitSubstringPathNotStrict)
go test ./internal/realtime/... -count=1                    ok
make test-it (Postgres 18 @ :5434)                          ok ./internal/handlers 4.2s
  incl. new: TestPurgeExpiredRefreshTokens,
             TestExportMissionDanglingVersion500 (+ version-less 200 control),
             TestOAuthLoginUnconfiguredRedirectsWithError
make ci-local-backend                                       PASS end-to-end — proves the new
  unit-test step (services/middleware/realtime) runs green in the mirrored order
cd apps/website/frontend && npm run build && npm run lint   PASS (tsc + vite, eslint clean;
  pre-existing chunk-size warning only)
editorconfig-checker .github/workflows/ci.yml Makefile      clean
```

### Notes / caveats

- **F2B-07 (Count error → 500):** the failure path is not fault-injectable against a live
  Postgres in the integration harness (the query is fixed and valid). Covered by compile +
  code review + existing list ITs on the success path. Extracting `missionListQuery` kept
  `ListMissions` under the cyclop-15 gate.
- **F2B-09 retention policy** (documented in `internal/services/token_purge.go`): rows are
  hard-deleted once `expires_at` is > 7 days past (`RefreshTokenRetention`), swept on boot +
  every 6 h (`StartRefreshTokenPurge`, stops with the shutdown signal context). Revoked rows
  are deliberately kept until expiry — they are the reuse-detection tripwire (T-126 S2).
- **F2B-11:** strict rate-limit selectors are now rooted `HasPrefix` matches; `main.go` had to
  change to `/api/v1/auth/` + `/api/v1/ingest/` — the old bare `/auth/` values would never
  match under exact prefixing (substring match was the only reason they worked).
- **F3-01 bounds:** 3 attempts total, `Retry-After` honored (fractional seconds), clamped to
  5 s max / 1 s default, context-aware wait, body replay via `req.GetBody`.
- **F3-03:** login redirects to the SPA with `#error=oauth_unconfigured`; copy added to
  `AUTH_ERROR_COPY` in `pages/auth.tsx` (unknown codes already fell back gracefully).
- **Pre-existing failure, out of scope:** repo-wide `make verify-editorconfig` fails on 4
  errors in `scripts/map-assets/lib/sap-seam-metrics.mjs` + `scripts/map-assets/vendor/bcdec.h`
  — tracked files from **T-090.1.2.2** (satellite program, locked for this ticket; present on
  `main`). The two files this batch touches (`ci.yml`, `Makefile`) pass the checker. T-090
  owner should fix or exclude (vendor header likely belongs in `.editorconfig-checker.json`).
