# T-125 — Claude Code handoff

**Status:** **in progress** · active slice **T-125.4** · doc prep @ `bcf179b` + spec expand  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md) §T-125.4 (expanded)  
**Authority:** [`CODING_STANDARDS.md`](../docs/platform/CODING_STANDARDS.md) §2 Go, §4 Errors, §5 Enfusion, §8 Size, §9 Logging, §10 matrix · [`DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md) §3 `@route`

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · T-125.3 @ `e5fbf4b` (tag **T-125.3**)

---

## T-125.3 — DONE ✓

Strict TS, eslint TS-2..7/LOG-2/COMP-1(TS), TS-6 `@model`/`@contract` on 36 FE exports. Do not redo.

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
    func (h *Handler) JWT()
    func (h *Handler) Discord()
    func (h *Handler) Webhook()

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
    • Optional enrichment joins → skip row on NotFound, continue
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

  Prefer: move DB touch in telemetry ingest into services/ (minimal extraction).
  If refactor too large: add GO-9 rows to .coding-standards-allowlist.yaml with expires + reason;
  document in Shipped note.

═══════════════════════════════════════════════════════════════════════════════
TASK 5 — ERR-4: scripts/website/verify-error-envelope.sh
═══════════════════════════════════════════════════════════════════════════════

  Assert every error JSON response uses ONLY keys {error, details}:
    • Scan handlers/ for c.JSON(http.Status*4xx*|*5xx*, gin.H{…})
    • Also StatusBadRequest, StatusNotFound, etc.
    • Fail on keys like message, err, errors, status in the gin.H literal
    • {error: err.Error()} on 500 is OK if key is exactly "error"

═══════════════════════════════════════════════════════════════════════════════
TASK 6 — LOG-3: scripts/website/verify-handler-logging.sh
═══════════════════════════════════════════════════════════════════════════════

  Heuristic: consequential 4xx/5xx handlers should log id + status + duration
  (see CreateVersion log.Printf pattern in missions.go).

  Expected-miss exemptions: bare 401 auth failures, simple 404 id lookups.

  Ship a working script (exit 1 on violation). If heuristic needs v1 carve-outs,
  document them in script header comments — prefer tightening over disabling.

═══════════════════════════════════════════════════════════════════════════════
TASK 7 — SIZE-1/3: scripts/website/verify-file-length.mjs
═══════════════════════════════════════════════════════════════════════════════

  Node script; read .coding-standards-allowlist.yaml for SIZE-2/SIZE-3 exemptions.

  Rules:
    • >600 lines → WARN (stderr, exit 0) — SIZE-1
    • >1000 lines → exit 1 unless allowlisted — SIZE-3

  Add SIZE-3 allowlist rows (if missing) for standing debt:
    apps/website/frontend/src/pages/admin.tsx      (~1628 L)
    apps/website/frontend/src/pages/doctrine.tsx     (~1288 L)
    apps/website/internal/handlers/events.go         (~1038 L)

  SIZE-2 tactical-map/** already allowlisted — skip SIZE-3 fail for those paths.

  Scan: apps/website/**.{go,ts,tsx} + apps/website/frontend/src/** (exclude node_modules, dist)

═══════════════════════════════════════════════════════════════════════════════
TASK 8 — ENF-4: validate.mjs Enfusion DTO branch
═══════════════════════════════════════════════════════════════════════════════

  Extend packages/tbd-schema/scripts/validate.mjs:

  Start with TBD_MissionSlotStruct.c (@contract mission.schema.json#/$defs/slot):
    • Add packages/tbd-schema/enfusion/mission-slot.sample.json (or registry/ path)
      — valid instance of $defs/slot
    • validate.mjs: "Enfusion DTO fixtures:" section — compile schema pointer, validate fixture

  Scan apps/mod/tbd-framework/Scripts/Game/TBD/Backend/*.c for @contract tags;
  each requires a matching golden JSON that validates.

  Do NOT broad-edit mod gameplay code. enfusion-mcp if touching .c files.

═══════════════════════════════════════════════════════════════════════════════
TASK 9 — Makefile + ci-local wiring
═══════════════════════════════════════════════════════════════════════════════

  Root Makefile — add:

  verify-coding-standards:
  	bash scripts/website/verify-handler-imports.sh
  	bash scripts/website/verify-error-envelope.sh
  	bash scripts/website/verify-handler-logging.sh
  	node scripts/website/verify-file-length.mjs

  Add $(MAKE) verify-coding-standards to ci-local (after ci-local-backend, before or with schema).

  GO-7 @route pass lives in verify-citations (Task 1) — already in ci-local-schema.

  Update Makefile help comment for verify-coding-standards (CODING_STANDARDS §11).

═══════════════════════════════════════════════════════════════════════════════
TASK 10 — Shipped note (only doc edit allowed)
═══════════════════════════════════════════════════════════════════════════════

  Append **Shipped (T-125.4):** to docs/platform/t125_coding_standards_enforcement.md §T-125.4:
    @route count added; M6 sites fixed; WriteAudit annotations; scripts added;
    allowlist rows; ENF-4 fixture; make ci-local wall-clock; any GO-9 debt allowlisted.

═══════════════════════════════════════════════════════════════════════════════
VERIFY — ALL MUST EXIT 0
═══════════════════════════════════════════════════════════════════════════════

  make verify-citations          # @contract + TS-6 + new GO-7 @route pass
  make verify-coding-standards   # new scripts bundle
  make schema-validate           # includes ENF-4 branch
  make test-it                   # no handler regressions
  make ci-local                  # full gate; report wall-clock

  cd apps/website/frontend && npm run build && npm run lint && npm test  # no FE regressions

═══════════════════════════════════════════════════════════════════════════════
COMMIT + TAG
═══════════════════════════════════════════════════════════════════════════════

  git add <explicit paths>
  git commit -m "$(cat <<'EOF'
T-125.4 platform: @route tags, M6 fixes, verify-* scripts, ENF-4 DTO gate

Complete @route on ~90 handlers + GO-7 verifier pass; fix 15 silent db.First
reads; GO-3 WriteAudit annotations; add verify-handler-imports/error-envelope/
logging/file-length scripts + make verify-coding-standards; ENF-4 Enfusion fixture
in validate.mjs; wire ci-local.

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
  3. M6: 15/15 db.First sites fixed? (list any deferred)
  4. WriteAudit: N annotated
  5. Scripts: list new files + any allowlist rows added
  6. ENF-4: fixture path + validate.mjs output
  7. GO-9: violations fixed vs allowlisted
  8. make test-it + make ci-local wall-clock
  9. Ready for T-125.5: yes/no

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
