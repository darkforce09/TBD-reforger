# T-125 — Claude Code handoff

**Status:** **in progress** · active slice **T-125.4** · **110% bar** (no deferrals) · spec @ current main  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md) §T-125.4 (110% expanded)  
**Authority:** [`CODING_STANDARDS.md`](../docs/platform/CODING_STANDARDS.md) §2 Go, §4 Errors, §5 Enfusion, §8 Size, §9 Logging, §10 matrix · [`DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md) §3 `@route`

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · T-125.3 @ `e5fbf4b` (tag **T-125.3**)

---

## T-125.3 — DONE ✓

Strict TS, eslint TS-2..7/LOG-2/COMP-1(TS), TS-6 `@model`/`@contract` on 36 FE exports. Do not redo.

---

## 110% bar (operator — non-negotiable)

Anything previously labeled “v1”, “deferred”, “post-ship”, or “local-only” is **in scope for T-125.4**:

| Gap | 110% requirement |
|-----|------------------|
| `ci.yml` | Backend job runs `make verify-coding-standards` (same as `ci-local`) |
| LOG-3 | **All 75 5xx** + **mutator 400/409/413** logged; script enforces both |
| GO-7 | `@route` presence **and** match to `Register()` method+path |
| ENF-4 | **All 10** Backend `@contract` DTOs → sample JSON + validate.mjs |
| M6 bucket B | Non-NotFound errors → `log.Printf` even when handler returns 200 |
| telemetry | `RefreshLeaderboard` failure → log even on 200 (today WriteAudit only) |

**Revised task list (matches Claude plan — holding for go-ahead):**

| # | Task | Key deliverable |
|---|------|-----------------|
| T1 | GO-7 route-match | 82 handlers; Register parse; missing/mismatch/**unwired** fail |
| T2 | M6 ×15 | Bucket A 500+log; bucket B log non-NotFound @ 200 |
| T3 | GO-3 ×15 | WriteAudit `//nolint:errcheck` + reason |
| T4 | GO-9 | `services.RefreshLeaderboard`; allowlist auth/realtime only; verify-handler-imports.sh |
| T5 | ERR-4 | verify-error-envelope.sh (awk, brace-balanced) |
| T6 | LOG-3 two-band | 75 5xx + mutator 400/409/413 (~subset of 79 4xx); awk mutator set |
| T7 | SIZE | verify-file-length.mjs (dep-free node) |
| T8 | ENF-4 ×10 | `enfusion/*.sample.json`; root = smallest golden |
| T9 | Wiring | ci-local + **ci.yml backend step** |
| T10 | Shipped note | No-deferral summary |

**Acceptance:** `make ci-local` green AND `ci.yml` passes on push. Scripts: grep/awk, dep-free Node.

---

## Copy this into a **new** Claude Code chat — T-125.4 ONLY

```
═══════════════════════════════════════════════════════════════════════════════
T-125.4 — @route completion, M6 error handling, verify-* scripts, ENF-4 DTO gate
Ticket program: platform coding standards enforcement
═══════════════════════════════════════════════════════════════════════════════

You are implementing ONLY slice T-125.4. Read first, then execute.

READ ORDER (authoritative):
  1. CLAUDE.md — §Status (T-125 block), §Conventions, §Verifying changes
  2. docs/platform/t125_coding_standards_enforcement.md — §T-125.4 (expanded task list)
  3. docs/platform/CODING_STANDARDS.md — §2 Go (GO-1..3, GO-7, GO-9), §4 ERR-4, §5 ENF-4,
     §8 SIZE-1/2/3, §9 LOG-3, §10 matrix + §10.1 script inventory, §11 verify replay
  4. docs/platform/DOCUMENTATION_STANDARDS.md — §3.1 @route grammar
  5. apps/website/internal/handlers/handlers.go — Register() is the route source of truth
  6. .ai/artifacts/t125_claude_code_handoff.md — this checklist

Do NOT implement T-125.5 (.editorconfig/Prettier) or T-125.6 (registry/CLAUDE hub sync).
Do NOT redo T-125.3 (strict TS, eslint, TS-6 @model — live @ e5fbf4b).

═══════════════════════════════════════════════════════════════════════════════
PREFLIGHT (repo root)
═══════════════════════════════════════════════════════════════════════════════

  ./scripts/ticket brief T-125
  # Expect: SLICE: T-125.4, TARGETS: website + mod (validate.mjs / ENF-4 only for mod)

  git log -1 --oneline
  # Expect doc sync bcf179b or code e5fbf4b T-125.3 on main

  make db-up
  nvm use && node -v    # Node 26

  # Baseline (should all pass before you start)
  cd apps/website/frontend && npm run build && npm run lint && npm test
  make verify-citations
  cd apps/website && golangci-lint run ./...

  # Count work remaining
  rg -c '@route' apps/website/internal/handlers/*.go    # ~5 today
  rg '_ = .*\.First\(' apps/website/internal/handlers  # 15 M6 sites
  rg '_ = services\.WriteAudit' apps/website/internal/handlers  # ~15 GO-3 sites

═══════════════════════════════════════════════════════════════════════════════
ALREADY SHIPPED — DO NOT REDO
═══════════════════════════════════════════════════════════════════════════════

  T-125.1  ci.yml + make ci-local @ 9792182
  T-125.2  golangci full gate @ 80c7f07 (0 issues; M6 db.First deferred to THIS slice)
  T-125.3  strict TS + eslint + TS-6 FE @model/@contract @ e5fbf4b

  TS-6 on types/api/index.ts is DONE (subsumes old spec line "expand @model on index.ts").

═══════════════════════════════════════════════════════════════════════════════
EXECUTION MODEL
═══════════════════════════════════════════════════════════════════════════════

  • Work on main (single-ticket mode)
  • One commit when done; tag T-125.4; Co-Authored-By trailer
  • Stage paths explicitly (ignore mod .rdb, worlds/*.ent, map-assets symlinks)
  • MAY edit: docs/platform/t125_coding_standards_enforcement.md §T-125.4 Shipped note ONLY
  • DO NOT edit: registry.json, CLAUDE.md, docs/TICKET_*.md, CODING_STANDARDS.md body/matrix,
    docs/platform/README.md (Cursor after slice)
  • Mod .c edits: ENF-4 only if unavoidable; use enfusion-mcp before editing Enfusion

═══════════════════════════════════════════════════════════════════════════════
TASK 1 — GO-7: @route on every HTTP handler + verifier extension
═══════════════════════════════════════════════════════════════════════════════

  Route authority: apps/website/internal/handlers/handlers.go Register()
  All API routes are registered on the /api/v1 group → @route paths MUST include /api/v1 prefix.

  Tag grammar (DOCUMENTATION_STANDARDS §3.1):
    @route GET /api/v1/missions
    @route POST /api/v1/event-missions/:emid/register

  Add @route to EVERY handler matching:
    func (h *Handler) <Name>(c *gin.Context)
  in apps/website/internal/handlers/*.go EXCLUDING *_test.go.

  EXCLUDE (not HTTP handlers):
    func (h *Handler) JWT() / Discord() / Webhook()
    Lowercase helpers (loadEvent, loadMission, loadPending, auditQuery, …) — not Register targets

  Already tagged (5 — use as examples):
    registry.go      ListRegistry       GET /api/v1/registry
    missions.go      CreateVersion      POST /api/v1/missions/:id/versions
    missions.go      GetVersion         GET /api/v1/missions/:id/versions/:vid
    missions.go      ExportMission      GET /api/v1/missions/:id/export
    field_tools.go   InjectMission      POST /api/v1/missions/:id/inject

  Extend packages/tbd-schema/scripts/verify-contract-citations.mjs — GO-7 pass:
    • Scan apps/website/internal/handlers/*.go (skip *_test.go)
    • For each func (h *Handler) Name(c *gin.Context), preceding /** */ Godoc MUST match /@route\s+(GET|POST|PUT|PATCH|DELETE)\s+\S+/
    • Exit 1: file:line: Name missing @route (GO-7)
    • **110% route-match:** parse handlers.go Register() — nested Group() prefixes +
      METHOD("path", …, h.HandlerName) → HandlerName → (METHOD, pathTemplate)
    • Fail: missing @route, @route mismatch vs wired route, **unwired handler** (tagged or
      exported Handler(c) but not in Register())
    • Keep existing passes: @contract resolve, TS-6 FE @model/@contract

  Godoc quality: keep GO-6 (revive exported) — comment starts with handler name; @route sits in block.

═══════════════════════════════════════════════════════════════════════════════
TASK 2 — M6 / GO-2: fix 15 silent _ = h.db.First(...).Error sites
═══════════════════════════════════════════════════════════════════════════════

  errcheck does NOT flag .Error field reads on GORM chains — these are silent bugs.

  Files/lines (grep-confirmed):
    dashboard.go     60, 94, 96, 98
    deployments.go   66
    events.go        525
    cms.go           187, 192
    me.go            180
    wiki.go          84
    missions.go      294, 383, 515
    approvals.go     107, 143

  Reference pattern: apps/website/internal/handlers/registry.go ListRegistry
    if err := h.db.First(&x, …).Error; err != nil {
      if errors.Is(err, gorm.ErrRecordNotFound) { …404…; return }
      …500…; return
    }

  Context-aware fixes:
    • Hydration after create/update where row MUST exist → 500 on unexpected error
    • Optional enrichment joins (Bucket B: dashboard 60/94/96/98, deployments 66) → skip on
      NotFound; **log.Printf on any other error** even if handler still returns 200
    • Never leave _ = when the struct is read afterward

  NO blanket //nolint on these reads.

═══════════════════════════════════════════════════════════════════════════════
TASK 3 — GO-3: annotate bare _ = services.WriteAudit(...)
═══════════════════════════════════════════════════════════════════════════════

  Grep: _ = services.WriteAudit in handlers/

  Every discarded WriteAudit MUST have:
    //nolint:errcheck // best-effort: <short reason>

  Or handle the error (rare). Files include admin.go, auth.go, cms.go, approvals.go,
  telemetry.go, me.go, field_tools.go.

═══════════════════════════════════════════════════════════════════════════════
TASK 4 — GO-1/GO-9: scripts/website/verify-handler-imports.sh
═══════════════════════════════════════════════════════════════════════════════

  New bash script (exit 1 on violation):
    • Scan apps/website/internal/handlers/*.go excluding *_test.go
    • Allowed internal imports:
        github.com/tbd-milsim/reforger-backend/internal/{services,models,middleware,contract,config}
    • Allow stdlib, gin, gorm, uuid, google/uuid, etc.

  KNOWN violations today (fix or allowlist — do NOT weaken rule silently):
    handlers.go  → auth, realtime
    telemetry.go → db
    auth.go, me.go → auth

  Prefer: extract telemetry DB → services.RefreshLeaderboard (GO-9); allowlist ONLY structural
  auth/realtime on handlers.go, auth.go, me.go (.coding-standards-allowlist.yaml + Shipped note).
  Do NOT allowlist telemetry db import without extraction.

═══════════════════════════════════════════════════════════════════════════════
TASK 5 — ERR-4: scripts/website/verify-error-envelope.sh
═══════════════════════════════════════════════════════════════════════════════

  Assert every error JSON response uses ONLY keys {error, details}:
    • Scan handlers/ for c.JSON(http.Status*4xx*|*5xx*, gin.H{…})
    • Use awk with brace-balanced gin.H parsing (portable; passes today as guard-only)
    • Fail on keys like message, err, errors, status in the gin.H literal
    • {error: err.Error()} on 500 is OK if key is exactly "error"

═══════════════════════════════════════════════════════════════════════════════
TASK 6 — LOG-3 (full, two-band): logHandlerErr + timing + script
═══════════════════════════════════════════════════════════════════════════════

  Add middleware/timing.go; mount first in Register().

  logHandlerErr(c, name, status, detail) → log.Printf with c.FullPath(), status, dur from middleware.

  **Band 1 — 5xx (75 sites):** 74 InternalServerError + 1 BadGateway (cms.go).
  logHandlerErr before each. CreateVersion: add on branches missing status=+dur= in prior 3 lines.

  **Band 2 — mutator 4xx:** log before 400 (validation/bind/details), 409, 413 on
  POST/PUT/PATCH/DELETE. Total 4xx error JSON in handlers today: 79; band 2 = mutator subset.
  Exempt: GET 404 id lookups, bare 401 auth, simple GET validation 400.

  **Operational 200:** telemetry RefreshLeaderboard failure → log.Printf (path, dur, err) even on 200.

  Script verify-handler-logging.sh (awk + grep-derived mutator set) — exit 1 if:
    • any 5xx lacks logHandlerErr/log.Printf with status= in preceding 3 lines
    • any band-2 400/409/413 on mutator lacks same
  Shipped note: "LOG-3: 5xx + mutator 400/409/413 enforced; GET miss 404 exempt."

═══════════════════════════════════════════════════════════════════════════════
TASK 7 — SIZE-1/3: scripts/website/verify-file-length.mjs
═══════════════════════════════════════════════════════════════════════════════

  Node script (dep-free — fs only); read .coding-standards-allowlist.yaml for SIZE-2/SIZE-3 exemptions.

  Rules:
    • >600 lines → WARN (stderr, exit 0) — SIZE-1
    • >1000 lines → exit 1 unless allowlisted — SIZE-3

  Add SIZE-3 allowlist rows (if missing) for standing debt:
    apps/website/frontend/src/pages/admin.tsx      (1628 L)
    apps/website/frontend/src/pages/doctrine.tsx   (1289 L)
    apps/website/internal/handlers/events.go       (1041 L)

  SIZE-2 tactical-map/** already allowlisted — skip SIZE-3 fail for those paths.

  Scan: apps/website/**.{go,ts,tsx} + apps/website/frontend/src/** (exclude node_modules, dist)

═══════════════════════════════════════════════════════════════════════════════
TASK 8 — ENF-4: all 10 Backend @contract DTO fixtures
═══════════════════════════════════════════════════════════════════════════════

  Extend packages/tbd-schema/scripts/validate.mjs — data-driven scan of
  apps/mod/tbd-framework/Scripts/Game/TBD/Backend/*.c for @contract tags.

  Required fixtures packages/tbd-schema/enfusion/ (filename → pointer in validate.mjs):
    slot, meta, faction, circle, shape, zone, role, group, orbatFaction, root.sample.json
    • Nine $defs fixtures: minimal valid instances
    • root.sample.json: copy smallest packages/tbd-schema/golden-missions/*.json

  Scan apps/mod/tbd-framework/Scripts/Game/TBD/Backend/*.c for @contract tags.
  Each fixture validates via Ajv. Do NOT broad-edit mod gameplay code.

═══════════════════════════════════════════════════════════════════════════════
TASK 9 — Makefile + ci-local + ci.yml
═══════════════════════════════════════════════════════════════════════════════

  Root Makefile — add verify-coding-standards target (4 scripts bundle).
  Wire into ci-local after ci-local-backend.

  **110% — edit .github/workflows/ci.yml:**
    backend job: step `make verify-coding-standards` after integration tests.
  GO-7 rides verify-citations (schema job); ENF-4 rides schema-validate.

  Update Makefile help comment for verify-coding-standards (CODING_STANDARDS §11).

═══════════════════════════════════════════════════════════════════════════════
TASK 10 — Shipped note (only doc edit allowed)
═══════════════════════════════════════════════════════════════════════════════

  Append **Shipped (T-125.4):** to docs/platform/t125_coding_standards_enforcement.md §T-125.4:
    no-deferral summary — @route count + Register cross-check; M6 15/15; WriteAudit N;
    LOG-3 band counts; script paths; allowlist rows; ENF-4 10/10; ci.yml step; ci-local wall-clock.

═══════════════════════════════════════════════════════════════════════════════
VERIFY — ALL MUST EXIT 0
═══════════════════════════════════════════════════════════════════════════════

  make verify-citations          # @contract + TS-6 + new GO-7 @route pass
  make verify-coding-standards   # new scripts bundle
  make schema-validate           # includes ENF-4 branch
  make test-it                   # no handler regressions
  make ci-local                  # full gate; report wall-clock
  # Confirm ci.yml backend job would pass (verify-coding-standards step present)

  cd apps/website/frontend && npm run build && npm run lint && npm test  # no FE regressions

═══════════════════════════════════════════════════════════════════════════════
COMMIT + TAG
═══════════════════════════════════════════════════════════════════════════════

  git add <explicit paths>
  git commit -m "$(cat <<'EOF'
T-125.4 platform: @route tags, M6 fixes, verify-* scripts, ENF-4 DTO gate

Complete @route on ~82 handlers + GO-7 verifier (Register cross-check); fix 15 silent db.First
reads; GO-3 WriteAudit annotations; add verify-handler-imports/error-envelope/
logging/file-length scripts + make verify-coding-standards; ENF-4 10 fixtures in validate.mjs;
wire ci-local + ci.yml backend step.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
  git tag T-125.4

  Do NOT run ./scripts/ticket advance-slice (operator/Cursor after report).

═══════════════════════════════════════════════════════════════════════════════
RETURN REPORT (paste back to operator for Cursor doc sync)
═══════════════════════════════════════════════════════════════════════════════

  1. Commit hash + tag T-125.4
  2. @route: N added (M total handlers checked)
  3. M6: 15/15 db.First sites fixed (Bucket B non-NotFound logged)
  4. WriteAudit: N annotated
  5. LOG-3: 5xx count + mutator 4xx count logged; script passes
  6. Scripts: list new files + allowlist rows
  7. ENF-4: 10/10 fixtures + validate.mjs output
  8. GO-7: @route count + Register cross-check passes
  9. GO-9: telemetry extracted; auth/realtime allowlisted only
  10. ci.yml: verify-coding-standards step added
  11. make test-it + make ci-local wall-clock
  12. Ready for T-125.5: yes/no

═══════════════════════════════════════════════════════════════════════════════
END T-125.4 PROMPT
═══════════════════════════════════════════════════════════════════════════════
```

---

## Slice order (remaining)

| # | Slice | Focus |
|---|-------|-------|
| 4 | **T-125.4** | `@route`, M6, verify-* scripts, Enfusion DTO ← **ACTIVE** |
| 5 | **T-125.5** | `.editorconfig` / Prettier |
| 6 | **T-125.6** | **cursor-docs** — registry shipped, final hub sync |

## Return to Cursor

After T-125.4 verify → paste post-ship report → Cursor updates registry, CODING_STANDARDS matrix (GO-7, scripts live), CLAUDE §Done, advance to T-125.5.
