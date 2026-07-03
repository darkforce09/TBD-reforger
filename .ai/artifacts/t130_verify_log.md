# T-130 verify log

Branch `ticket/T-130` (worktree `.ai/artifacts/worktrees/TBD-T-130`). Sections per batch; Cursor flips tracker rows on doc sync.

---

## Batch 2 — T-130.4 / T-130.5 / T-130.6 (2026-07-03)

Base: merged `main` @ `47a4df71` (brings T-090.1.2.8) onto Batch 1.

**Commits:**

| Slice | SHA | Findings |
|-------|-----|----------|
| T-130.4 | `b62a66b7` | F1-16, F1-17, F1-18, F1-19, F1-20 |
| T-130.5 | `bb40a61a` | F4-03 (closes PARTIAL), F4-07, F2F-07 |
| T-130.6 | `c8b2fd6e` | F2B-05, F4-04 |

Tag **T-130** @ `90c9f261` (merged to main 2026-07-03).

---

## T-130.7 — Doc sync (2026-07-03)

**Executor:** cursor-docs · **Merge base:** `90c9f261`

| ID | Fix |
|----|-----|
| F1-09 | `everon/manifest.json` + unified golden `metersPerPixel` → **2** |
| F1-11 | `terrainId` schemas aligned (`minLength: 1`) |
| F5-08 | `tileUrl.ts` `tmsY` → `xyzRow` |
| F5-09 | Mermaid `<br/>` in t092 spec |
| F2C-04 | `./scripts/ticket brief` hybrid execution line |
| F4-08 | `ux_spec.md` §Keyboard shortcuts |
| F5-10 | Deferred (trivial spelling) |

```
./scripts/ticket sync && ./scripts/ticket check     PASS
cd apps/website/frontend && npm test                tileUrl tests PASS
make schema-validate                                PASS (11/11 gates)
```

Living tracker: [fable_5_omni_audit_report.md](fable_5_omni_audit_report.md) — OPEN/PARTIAL → RESOLVED.

---

### Verification run

```
go build ./...                                              PASS
gofmt -l internal cmd                                       empty
golangci-lint run ./...                                     0 issues
go test ./internal/services|middleware|realtime -count=1    ok / ok / ok
make test-it (Postgres 18 @ :5434)                          ok ./internal/handlers 4.99s
  new: TestMissionArchiveLifecycle (archive→global hidden/mine listed→
       status=live 400→unarchive draft→idempotent),
       TestMissionArchiveBlockedByUpcomingEvent (409 archive w/ upcoming,
       409 delete w/ any attachment, detach→archive 200),
       TestMissionSoftDelete (403 non-author, 204, 404 after, soft row kept)
cd apps/website/frontend: npm run build                     PASS (tsc + vite)
npm run lint                                                clean (complexity cap kept via
  MissionLifecycleActions extraction)
npm test                                                    43/43 (38 pre-existing +
  5 new editorSession.test.ts F4-03 marker cases)
```

### T-130.4 — mod robustness notes

- `FileHandle` API confirmed via enfusion-mcp `api_search` (offline index):
  `proto int Write(void data, int length=-1)`, `proto int GetLength()` — write checks
  compare bytes written `<= 0`.
- New shared `Scripts/WorkbenchGame/TBD_ExportPaths.c`: `PROFILE_WIN` single source
  (no env-var surface exists in Workbench plugins — documented at the constant) +
  `TBD_ExportJson.Escape` (\\ " \n \r \t) + checked `TBD_ExportJson.Write`.
- Raster/JSONL flush loops abort AND delete the partial output file on a failed write;
  meta-write failures delete the partial meta (raster/JSONL kept).
- Mission list RPC is now admin-gated (mirrors `TBD_RpcAsk_SelectMission`) and capped
  at 100 lines with an "… and N more" trailer; selection numbering unchanged.
- **Workbench compile/run smoke PENDING OPERATOR** — Workbench was not up on this rig
  (`mcp-smoke` timed out). Changes are mechanical guards (no export math/behavior
  change), but the plugins should be compiled + one export re-run before the next
  Workbench session relies on them.

### T-130.5 — F4-03 manual proof steps (documented for operator; logic vitest-covered)

1. `make api` + `make web`, dev-login as mission_maker, open `/missions/:id/edit`.
2. Make a local edit (place a slot); Save Version — or force the conflict first:
   edit locally, F5 with a differing server version → conflict dialog → **Load saved
   version**.
3. Open a **NEW tab** on the same `/missions/:id/edit` → editor boots with **no
   conflict prompt** (localStorage `tbd-editor-adopted:{missionId}` semver matches the
   server's current version).
4. Control: in the conflict dialog choose **Keep local draft** instead → new tab →
   conflict prompt **does** appear (marker cleared — genuine divergence still prompts).
5. Control 2: another author/tab saves a NEWER version server-side → new tab →
   conflict prompt appears (marker semver ≠ server semver).

Non-UUID trap (F4-07): open `/missions/not-a-uuid/edit` → full-bleed "Invalid mission
link" overlay blocks all editor interaction, links to Mission Library; no doomed GET,
no misleading "could not load" toast.

### T-130.6 — decisions documented

- **"mine" scope keeps archived missions** (badged, unarchive in dossier) — chose this
  over a `?include_archived` query param: no API surface growth, matches the
  announcements model. Global scope never lists archived (even the caller's own).
- Archive 409-guards only on **upcoming** event attachments; delete 409s on **any**
  attachment (past ORBAT/registration history must not lose its mission row —
  archive is the "hide it" tool, delete stays for never-used missions).
- Unarchive lands on **draft**, not the prior status — a formerly live mission
  re-enters review instead of silently going live.

### Pre-existing failures, out of scope (T-090 territory — do NOT fix in T-130)

- `make verify-editorconfig`: 4 errors in `scripts/map-assets/{lib/sap-seam-metrics.mjs,
  vendor/bcdec.h}` (tracked since T-090.1.2.2; unchanged).
- `npm run format:check`: 3 files under `src/features/tactical-map/layers/`
  (`satelliteUnified.ts/.test.ts`, `useTerrainBasemapLayer.ts`) arrived unformatted with
  the T-090.1.2.8 merge from main. My touched files pass Prettier.

---

## Batch 1 — T-130.1 / T-130.2 / T-130.3 (2026-07-02)

**Commits:**

| Slice | SHA | Findings |
|-------|-----|----------|
| T-130.1 | `6426600f` | F2B-07, F2B-08, F2B-09, F2B-11 |
| T-130.2 | `9db1b9e1` | F3-01, F3-02, F3-03 |
| T-130.3 | `755a889b` | F2B-06 |

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
