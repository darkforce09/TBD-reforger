# T-125 — Coding standards + 11/10 enforcement

**Ticket:** T-125 · **Program:** platform · **Status:** **ready** (T-124 shipped @ `cd11db0`)  
**Depends on:** T-124 (met) · **Active slice:** T-125.5 · **Handoff:** [`.ai/artifacts/t125_claude_code_handoff.md`](../../.ai/artifacts/t125_claude_code_handoff.md)

## In one sentence

Author **`CODING_STANDARDS.md`** (code style/structure/errors/tests — distinct from contract **documentation** standards) and enforce it repo-wide with a full CI gate, hardened linters, TypeScript `strict: true`, complete handler `@route` tags, and error-handling policy.

## Authority split

| Doc | Owns |
|-----|------|
| [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) | `@contract` / `@route` / Godoc / TSDoc / Enfusion authority tags |
| **`CODING_STANDARDS.md`** (new) | Style, structure, errors, tests, file size, TS strict, Go linter policy, formatting |

Cross-link both from [`docs/platform/README.md`](README.md) and [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md).

---

## Slice plan

| Slice | Executor | Scope |
|-------|----------|--------|
| **T-125.0** | claude-code | Write `CODING_STANDARDS.md` |
| **T-125.1** | claude-code | `ci.yml` + Postgres 18 service + `make ci-local` |
| **T-125.2** | claude-code | golangci full set + fix all Go lint |
| **T-125.3** | claude-code | TS `strict: true` + eslint tag enforcement + fixes |
| **T-125.4** | claude-code | `@route` completion, error-handling, Enfusion DTO fixture gate |
| **T-125.5** | claude-code | `.editorconfig` + Prettier + FMT-2/FMT-3 CI gates |
| **T-125.6** | cursor-docs | Registry shipped, hub links, CLAUDE §Done, `./scripts/ticket sync` |

Advance after each slice verifies: `./scripts/ticket advance-slice T-125`

**Execution:** commits on `main` (single-ticket mode) unless operator prefers `./scripts/ticket run` on a branch.

---

## T-125.0 — Author CODING_STANDARDS.md

Minimum sections:

- **Go:** no silent `_ =` on DB/audit without explicit rationale; handler vs `services/` boundaries; when integration tests are required
- **TS:** `"strict": true` in [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json); pages vs `features/`; god-file limits (admin/doctrine split guidance)
- **Errors:** `{ error }` contract, status code table, validation `details[]`
- **Formatting:** `.editorconfig`, optional Prettier for TS/CSS (Go: `gofmt`/`goimports`)
- **Testing:** minimum bar per layer (Go IT for handlers; FE tests for `features/` hooks/utils)
- **Relationship** to DOCUMENTATION_STANDARDS (docs vs code comments)

**Verify:** doc renders; cross-links valid.

---

## T-125.1 — Primary CI workflow

New [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) — **required on every PR/push to `main`:**

| Job | Steps |
|-----|-------|
| **backend** | Postgres 18 service (`postgres:18-alpine`, creds `tbd/tbd`; CI reaches it at `localhost:5432` — local dev uses host `5434` via compose); Go **1.26**; gofmt (FMT-1), `go build`, `make test-it` |
| **frontend** | Node **26**; `npm ci`, `npm run lint`, `npm run build`, `npm test` |
| **schema** | `npm run validate`, `make verify-citations` |

Add **`make ci-local`** (or `make check`) mirroring CI.

**Shipped (T-125.1):** [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) — three jobs
(**backend** `postgres:18-alpine` + Go **1.26** → gofmt (FMT-1) + `go build` + `make test-it`;
**frontend** Node **26** → `npm ci` + lint + build + test; **schema** → `npm run validate` +
verify-citations), required on every push/PR to `main` (no path filter). Local mirror:
**`make ci-local`** (sub-targets `ci-local-{backend,frontend,schema}`). `contracts.yml` /
`schema.yml` stay as path-scoped supplements; golangci hardening + `only-new-issues` removal is **T-125.2**.

**Verify:** ✅ `make ci-local` green locally (backend needs `make db-up`); `ci.yml` required on `main`.

---

## T-125.2 — golangci full gate

Harden [`apps/website/.golangci.yml`](../../apps/website/.golangci.yml):

- Add **errcheck**, **govet**, **staticcheck** (in addition to revive `exported`)
- **Remove `only-new-issues: true`** from [`contracts.yml`](../../.github/workflows/contracts.yml) (or merge golangci into `ci.yml` and dedupe)
- Fix **all** linter findings repo-wide

**Verify:** `golangci-lint run ./...` clean; `make test-it`.

**Shipped (T-125.2):** [`apps/website/.golangci.yml`](../../apps/website/.golangci.yml) enables
**revive** (`exported`), **errcheck** (`check-blank: true`), **errorlint**, **staticcheck**, **govet**,
and **cyclop** (`max-complexity: 15`). Exclusions: `node_modules` (vendored Go) + generated
`internal/contract/`, and `_test.go` exempt from errcheck/cyclop (fixtures discard known-good errors;
integration tests are linear — §2 GO-2/3 + COMP-1 target production logic). **`only-new-issues`
removed** from [`contracts.yml`](../../.github/workflows/contracts.yml) (now a path-filtered
supplement); golangci wired into [`ci.yml`](../../.github/workflows/ci.yml) backend (after gofmt,
before build) and `make ci-local-backend`, with the **CI-1** grep guard. **57 findings fixed**
repo-wide: errcheck 34 → best-effort `//nolint:errcheck`; revive 12 → const-block Godoc; errorlint 7
→ `errors.Is`; cyclop 3 → `//nolint:cyclop` (events/cms/missions handlers — splits are SIZE-3/T-125.4);
staticcheck 1 → `fmt.Fprintf`. Result: `golangci-lint run ./...` **0 issues**, `make test-it` green,
`make build` clean. New [`.coding-standards-allowlist.yaml`](../../.coding-standards-allowlist.yaml)
(SIZE-2 MC-perf stub). Note: the M6 `_ = db.First(...).Error` reads are a struct **field** access (not
a func call) so errcheck does not flag them — they stay **T-125.4** (which owns `_ = db.First` fixes).

**T-125.2.1:** ci.yml step-order comment + CI-1 moved to verify-ci1.sh for §G forbidden-rg.

---

## T-125.3 — TypeScript strict + eslint tags

- Enable **`strict: true`** in [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json); fix all errors (expect MC + pages touch)
- Harden [`eslint.config.js`](../../apps/website/frontend/eslint.config.js): enforce **`@contract` / `@model`** on cross-boundary exports (custom rule or extend [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs))

**Verify:** `npm run build && npm run lint && npm test`.

**Shipped (T-125.3):**
- **TS-1** — `strict: true` in both [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json)
  and [`tsconfig.node.json`](../../apps/website/frontend/tsconfig.node.json) (`npm run build` = `tsc -b`
  builds both). **0 tsc errors** — the codebase was already strict-clean.
- **eslint** ([`eslint.config.js`](../../apps/website/frontend/eslint.config.js)) — added
  `@typescript-eslint/no-explicit-any` + `no-non-null-assertion` (**TS-3**), `no-empty
  {allowEmptyCatch:false}` + `no-empty-function` (**TS-4/TS-7**), `no-console {allow:[warn,error]}`
  (**LOG-2**), `complexity {max:15}` (**COMP-1** TS half), and **TS-2** layer boundaries via
  **`eslint-plugin-import-x`** `import-x/no-restricted-paths` (`features/` + `components/` ✗ `pages/`)
  plus built-in `no-restricted-imports` for the `@/pages` alias form.
- **Fallout fixed (50):** 18 non-null assertions (real fixes — a `mustGet` Y.Map helper that throws
  on a broken invariant, null guards, `?? []`), 6 empty functions (documented noop / promise-chain
  continuation), 5 dev `console` (one → `console.warn`; four dev diagnostics keep their
  `import.meta.env.DEV` guard + an inline `no-console` opt-out), 21 `complexity` opt-outs (inline
  `// eslint-disable-next-line complexity` with a per-function reason on MC hot paths + page render
  functions — no refactor, mirroring the Go `//nolint:cyclop` approach).
- **TS-6** — [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs)
  extended: every exported `interface`/`type` in `types/`, `api/`, `hooks/` (excl. generated
  `types/contract/**`) MUST carry `@model` or `@contract`; generic envelopes (`Paginated<T>`) are
  exempt. **23 tags added** (36 exports checked); the existing 24 `@contract` citations still resolve.
- **Verify:** `npm run build` / `npm run lint` / `npm test` (**21/21**) clean; `make verify-citations`
  exit 0; `make ci-local` green (golangci **0 issues**, `go build`, `make test-it` ok, schema validate).
  New devDep **`eslint-plugin-import-x`** — `eslint-plugin-import@2.32` peers eslint ≤9 and is
  incompatible with eslint 10.6.

---

## T-125.4 — Routes, errors, DTO gate

**Goal:** Close the remaining **Go-side CI-SCRIPT** gaps in CODING_STANDARDS §10 — handler `@route`
tags (GO-7), silent DB reads (M6 / GO-2), best-effort audit rationale (GO-3), import gate (GO-9),
error envelope (ERR-4), consequential error logging (LOG-3: 5xx + mutator 4xx), file length (SIZE-1/3),
and Enfusion DTO fixtures (ENF-4). Wire **`make verify-coding-standards`** into **`make ci-local`**
**and** **`ci.yml` backend job**.

**110% bar:** Every rule in this slice must be **enforced in CI** (`ci.yml` + `make ci-local`), not merely
documented or locally runnable. No “follow-up” deferrals for items listed below.

**Revised task list (110% — Claude plan, no deferrals):**

| # | Task | Deliverable |
|---|------|-------------|
| T1 | GO-7 route-match | `@route` on all **82** handlers; verifier parses `Register()` (group prefixes + `METHOD("path",…,h.Name)`) and asserts each `@route` equals wired method+path — **missing / mismatch / unwired all fail** |
| T2 | M6 ×15 | Bucket A → 500+log; bucket B → log non-NotFound even at 200 |
| T3 | GO-3 ×15 | `//nolint:errcheck // best-effort: …` on every discarded `WriteAudit` |
| T4 | GO-9 | Extract `services.RefreshLeaderboard`; allowlist only structural `auth`/`realtime`; `verify-handler-imports.sh` (grep + allowlist YAML) |
| T5 | ERR-4 | `verify-error-envelope.sh` (awk, brace-balanced `gin.H` scan) |
| T6 | LOG-3 two-band | Timing + `logHandlerErr` (`c.FullPath()`); band 1 = **75** 5xx; band 2 = mutator **400/409/413** (~subset of **79** total 4xx sites); script (awk + grep-derived mutator set); telemetry refresh logs on 200 path |
| T7 | SIZE | `verify-file-length.mjs` (dep-free Node); SIZE-3 rows `admin.tsx` / `doctrine.tsx` / `events.go` |
| T8 | ENF-4 ×10 | `enfusion/{slot,meta,faction,circle,shape,zone,role,group,orbatFaction,root}.sample.json`; root = copy smallest golden; `validate.mjs` filename→pointer |
| T9 | Wiring | `make verify-coding-standards` → `ci-local` **and** new **`ci.yml` backend step** after integration tests; GO-7/ENF-4 ride schema job |
| T10 | Shipped note | No-deferral summary (only doc edit Claude may append) |

**Acceptance:** `make ci-local` green **and** `ci.yml` passes on push. Scripts portable (**grep/awk**, dep-free Node).

**User decisions (locked):**
1. **LOG-3 full** — `logHandlerErr` on **every 5xx** (75 sites) **and** consequential **4xx** on mutators
   (**400** with validation/`details`, **409**, **413**). Script enforces both bands. Operational failures
   that return 200 but matter (e.g. `RefreshLeaderboard` after ingest) get `log.Printf` anyway.
2. **GO-9** — extract `telemetry.go` → `services.RefreshLeaderboard`; allowlist structural
   `auth`/`realtime` on `handlers.go`, `auth.go`, `me.go` (YAML + note for CODING_STANDARDS GO-9 row).
3. **CI** — wire **`make verify-coding-standards`** into **`ci.yml` backend job** AND **`make ci-local`**
   (GitHub and local must match — no “local-only” acceptance).

**Authority:** [`handlers.go` `Register()`](../../apps/website/internal/handlers/handlers.go) (all paths
under `/api/v1/…`). `@route` grammar: [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §3.1.

### Task 1 — GO-7: `@route` on every HTTP handler

- **Scope:** **82** exported `func (h *Handler) <Name>(c *gin.Context)` (exclude `*_test.go`).
  **Exclude:** `JWT()` / `Discord()` / `Webhook()`; lowercase helpers returning values
  (`loadPending`, `auditQuery`, `loadEvent`, `loadEventMission`, `loadMission`).
- **Today:** **5 tagged** → **~77 to add** (`registry.go`, `missions.go` ×3, `field_tools.go` ×1).
- **Verifier** ([`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs)):
  match `^func \(h \*Handler\) ([A-Z]\w*)\(c \*gin.Context\) \{$`; preceding Godoc MUST contain
  `@route (GET|POST|PUT|PATCH|DELETE) /api/v1/…`.
  **110% route-match (not presence-only):** parse [`Register()`](../../apps/website/internal/handlers/handlers.go)
  — walk nested `Group()` prefixes + each `METHOD("path", …, h.HandlerName)` — build
  `HandlerName → (METHOD, pathTemplate)` including `/api/v1` prefix. Fail on:
  - **missing** `@route` on a wired handler;
  - **mismatch** (Godoc method/path ≠ wired route);
  - **unwired** (handler has `@route` or matches `func (h *Handler) Name(c *gin.Context)` but is not
    registered in `Register()`).
  Rides `make verify-citations` (CI **schema** job).

### Task 2 — M6 / GO-2: fix 15 silent `_ = h.db.First(…).Error`

**Bucket A — required hydration (500 on unexpected error):** `missions.go` 294/383/515,
`events.go` 525, `approvals.go` 107/143, `cms.go` 187/192, `me.go` 180, `wiki.go` 84 (post-upsert).

**Bucket B — optional enrichment (NotFound OK, log unexpected):** `dashboard.go` 60/94/96/98,
`deployments.go` 66 — use `errors.Is(NotFound)` continue; **`log.Printf` on non-NotFound errors** even
when the handler still returns 200.

Reference: [`registry.go` `ListRegistry`](../../apps/website/internal/handlers/registry.go). Pair bucket-A
500s with `logHandlerErr` (Task 6). **No blanket `//nolint`.**

### Task 3 — GO-3: bare `_ = services.WriteAudit(…)` 

Audit M6 also covers audit drops. Every discarded `WriteAudit` MUST carry
`//nolint:errcheck // best-effort: <why safe>` (GO-3). Grep `handlers/` for `_ = services.WriteAudit`
and annotate or handle (~15 sites across `admin.go`, `auth.go`, `cms.go`, `approvals.go`, `telemetry.go`,
`me.go`, `field_tools.go`).

### Task 4 — GO-9: `services.RefreshLeaderboard` + `verify-handler-imports.sh`

- **Extract:** new [`services/leaderboard.go`](../../apps/website/internal/services/leaderboard.go)
  wrapping `internal/db.RefreshLeaderboard`; `telemetry.go` drops `internal/db` import.
- **Script:** [`verify-handler-imports.sh`](../../scripts/website/verify-handler-imports.sh) — allowed
  internal: `services|models|middleware|contract|config`; read GO-9 rows from allowlist YAML.
- **Allowlist (structural):** `handlers.go` (auth, realtime), `auth.go`, `me.go`.

### Task 5 — ERR-4: `verify-error-envelope.sh`

New script ([`verify-error-envelope.sh`](../../scripts/website/verify-error-envelope.sh)): scan
`handlers/` for `c.JSON(http.Status4xx|5xx, gin.H{…})` (and named constants like `StatusBadRequest`).
Use **awk with brace-balanced** `gin.H{…}` parsing (portable; no Node dep). Assert top-level keys ⊆
**`{error, details}`** only (`message`, `err`, `errors`, `status` as body keys fail). Document rare
false-positive carve-outs in script header. Baseline today: **passes without handler edits** (guard only).

### Task 6 — LOG-3 (full, two-band): helper + timing + script

- **`middleware/timing.go`** + mount at top of `Register()`.
- **`logHandlerErr`** uses `c.FullPath()` (not `c.Param("id")`):

```go
log.Printf("%s: path=%s status=%d %s dur=%s", name, c.FullPath(), status, detail, dur)
```

- **Band 1 — 5xx (75 sites):** `logHandlerErr` immediately before every `InternalServerError` (**74**)
  + `BadGateway` (**1**, `cms.go`). CreateVersion: keep existing logs; **still** add `logHandlerErr` on
  any 500 branch that lacks `status=` + `dur=` in the preceding 3 lines.
- **Band 2 — mutator 4xx:** on POST/PUT/PATCH/DELETE handlers, log before **400** (validation/bind/
  `details`), **409**, **413**. Total **4xx** error responses in handlers today: **79**; band 2 is the
  mutator subset (~not every GET **400**). **Exclude:** bare GET **404** id lookups, auth **401**s,
  simple enum/body **400** on read-only GETs.
- **Operational (200 but failed side-effect):** `telemetry.go` `RefreshLeaderboard` failure — add
  `log.Printf` with `path=` + `dur=` even though the handler returns 200 (today only WriteAudit).
- **Script** [`verify-handler-logging.sh`](../../scripts/website/verify-handler-logging.sh): portable
  **awk + grep-derived mutator set** (from `@route` tags or Register table). Exit 1 if:
  - any **5xx** `c.JSON` lacks `logHandlerErr(` / `log.Printf` with `status=` in preceding 3 lines;
  - any band-2 **400/409/413** on a mutating handler lacks the same.
- **Shipped note:** *"LOG-3: 5xx + mutator 400/409/413 enforced; GET miss 404 exempt."*

### Task 7 — SIZE-1 / SIZE-3: `verify-file-length.mjs`

New [`scripts/website/verify-file-length.mjs`](../../scripts/website/verify-file-length.mjs) —
**dep-free Node** (read allowlist YAML via `fs`, no npm deps):

- **>600 lines** → WARN to stderr (SIZE-1, exit 0).
- **>1000 lines** → exit 1 (SIZE-3) unless path matches [`.coding-standards-allowlist.yaml`](../../.coding-standards-allowlist.yaml).
- **Standing debt** (add SIZE-3 allowlist rows if not present):

  | File | Lines | Split plan |
  |------|------:|------------|
  | `apps/website/frontend/src/pages/admin.tsx` | ~1628 | admin sub-surfaces |
  | `apps/website/frontend/src/pages/doctrine.tsx` | 1289 | wiki split-pane helpers |
  | `apps/website/internal/handlers/events.go` | 1041 | ORBAT → `services/` (GO-1) |

- SIZE-2 MC allowlist (`tactical-map/**`) already in YAML — honour it.

### Task 8 — ENF-4: all Backend `@contract` DTO fixtures

**10 Enfusion DTO `@contract` tags** under `Scripts/Game/TBD/Backend/` (scan, do not hand-maintain):

| Struct | Pointer |
|--------|---------|
| `TBD_MissionSlotStruct` | `#/$defs/slot` |
| `TBD_MissionMetaStruct` | `#/$defs/meta` |
| `TBD_MissionFactionStruct` | `#/$defs/faction` |
| `TBD_MissionCircleStruct` | `#/$defs/circle` |
| `TBD_MissionShapeStruct` | `#/$defs/shape` |
| `TBD_MissionZoneStruct` | `#/$defs/zone` |
| `TBD_MissionOrbatRoleStruct` | `#/$defs/role` |
| `TBD_MissionOrbatGroupStruct` | `#/$defs/group` |
| `TBD_MissionOrbatFactionStruct` | `#/$defs/orbatFaction` |
| `TBD_MissionDocumentStruct` | `#/` (root) |

- Add **`packages/tbd-schema/enfusion/*.sample.json`** — fixed filenames (data-driven
  `validate.mjs` maps filename → schema pointer):

  | Fixture | Schema pointer | Authoring |
  |---------|----------------|-----------|
  | `slot.sample.json` | `#/$defs/slot` | minimal valid instance |
  | `meta.sample.json` | `#/$defs/meta` | minimal valid instance |
  | `faction.sample.json` | `#/$defs/faction` | minimal valid instance |
  | `circle.sample.json` | `#/$defs/circle` | minimal valid instance |
  | `shape.sample.json` | `#/$defs/shape` | minimal valid instance |
  | `zone.sample.json` | `#/$defs/zone` | minimal valid instance |
  | `role.sample.json` | `#/$defs/role` | minimal valid instance |
  | `group.sample.json` | `#/$defs/group` | minimal valid instance |
  | `orbatFaction.sample.json` | `#/$defs/orbatFaction` | minimal valid instance |
  | `root.sample.json` | `#/` (root document) | **copy smallest** [`golden-missions/`](../../packages/tbd-schema/golden-missions/) mission |

- Extend [`validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs): scan
  `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/*.c` for `@contract`; require matching fixture +
  Ajv subschema compile. **No `.c` edits** if tags exist.

### Task 9 — Makefile + `ci-local` + **`ci.yml`**

In root [`Makefile`](../../Makefile):

```makefile
verify-coding-standards: ## GO-1/9, ERR-4, LOG-3, SIZE-1/3 script bundle
	bash scripts/website/verify-handler-imports.sh
	bash scripts/website/verify-error-envelope.sh
	bash scripts/website/verify-handler-logging.sh
	node scripts/website/verify-file-length.mjs
```

Wire into **`ci-local`** after **`ci-local-backend`**, before **`ci-local-schema`**.

**Also edit [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml):**
- **backend** job: add step `make verify-coding-standards` after integration tests (mirrors `ci-local`).
- GO-7 + ENF-4 already ride **schema** job (`verify-citations`, `npm run validate`).

Note: **`@model` on `types/api/index.ts`** completed in **T-125.3**.

**Verify:** `make verify-citations` · `make verify-coding-standards` · `make schema-validate` ·
`make test-it` · `make ci-local` · FE build/lint/test · `golangci-lint run ./...` stays 0 ·
**push to `main` would pass `ci.yml`** (local replay = GitHub).

**Out of scope:** Prettier (T-125.5); registry/CLAUDE hub prose (T-125.6); TS/eslint (T-125.3).

### Task 10 — Shipped note (Claude only doc edit)

Append **`Shipped (T-125.4):`** under §T-125.4 with a no-deferral summary: `@route` count + Register
cross-check; M6 15/15; WriteAudit annotations; LOG-3 two-band counts; script paths; allowlist rows;
ENF-4 10/10 fixtures; `ci.yml` backend step; `make ci-local` wall-clock.

**Shipped (T-125.4)** — no deferrals; `make ci-local` green @ **22s** (Node 26; golangci 0 issues, FE 21/21):
- **GO-7 route-match:** **77** `@route` tags added (82 handlers total); `verify-contract-citations.mjs`
  parses `Register()` and asserts every `@route` **method+path matches the wired route** — "Checked 82
  handler(s) against Register() routes". Rides the schema CI job.
- **M6 / GO-2:** **15/15** silent `_ = h.db.First(…).Error` fixed — **8 bucket-A** (must-exist reload →
  500 + log) + **7 bucket-B** (enrichment → log non-`NotFound`, continue at 200). `GetDashboard` carries a
  documented `//nolint:cyclop` (4 M6 guards push it past 15).
- **GO-3:** WriteAudit discards were already `//nolint:errcheck` (T-125.2) — **0 new**, 15 sites verified.
- **LOG-3 (two-band):** `middleware.Timing()` + `logHandlerErr` (path=`c.FullPath()`); **140** call sites
  (**75** band-1 5xx + **65** band-2 mutator 400/409/413); `IngestMatchResults` logs a `RefreshLeaderboard`
  failure on its 200 path. `verify-handler-logging.sh` (POSIX awk + Register-derived mutator set) enforces both.
- **GO-9:** `services.RefreshLeaderboard` extracted (telemetry drops `internal/db`); `verify-handler-imports.sh`
  + **3 structural** GO-9 allowlist rows (`handlers.go` auth+realtime, `auth.go`, `me.go`).
- **ERR-4:** `verify-error-envelope.sh` ({error, details} only) — caught + fixed `field_tools.go` 422
  `solution`→`details` (2 sites; FE unaffected).
- **SIZE:** `verify-file-length.mjs` (dep-free); SIZE-3 rows `admin.tsx`/`doctrine.tsx`/`events.go`;
  3 warns (events.tsx/operations.tsx/missions.go), 0 violations.
- **ENF-4:** **10/10** `packages/tbd-schema/enfusion/*.sample.json` (slot, meta, faction, circle, shape,
  zone, role, group, orbatFaction, root) + data-driven `validate.mjs` branch.
- **Wiring:** `make verify-coding-standards` → `ci-local` **and** the `ci.yml` backend job (after TEST-1).

---

## T-125.5 — Repo hygiene (FMT-2 + FMT-3)

**Goal:** Ship the two remaining **Readability** formatting gates from CODING_STANDARDS §7 — root
**`.editorconfig`** (FMT-2) and **Prettier** for TS/TSX/CSS (FMT-3) — wired into **`make ci-local`**
and **`ci.yml`**. This closes the last CI-BLOCK rules that were **planned** after T-125.4.

**Authority:** [`CODING_STANDARDS.md`](CODING_STANDARDS.md) §7 FMT-2/FMT-3, §10 matrix, §11 verify replay.

**Baseline today:** No root `.editorconfig`, no Prettier config/scripts in
[`apps/website/frontend/package.json`](../../apps/website/frontend/package.json). Existing FE style
is **2-space**, **single quotes**, **no semicolons** (match Prettier to current code, not a style
revolution). Go stays **gofmt/tabs** (FMT-1 already live — do not add Prettier for `.go`).

### Task list

| # | Task | Deliverable |
|---|------|-------------|
| T1 | FMT-2 `.editorconfig` | Root [`.editorconfig`](../../.editorconfig): UTF-8, LF, final newline, trim trailing WS; **tabs** for Go; **2-space** for TS/JS/JSON/YAML/MD/CSS |
| T2 | FMT-2 checker | `editorconfig-checker` from repo root in **`ci-local`** + **`ci.yml`** (exclude `node_modules`, `dist`, generated contract, mod binaries/LFS) |
| T3 | FMT-3 Prettier | `prettier` + `eslint-config-prettier` devDeps; [`.prettierrc`](../../apps/website/frontend/.prettierrc) + [`.prettierignore`](../../apps/website/frontend/.prettierignore) |
| T4 | FMT-3 scripts | `npm run format` + `npm run format:check` in frontend `package.json` |
| T5 | ESLint compat | Extend [`eslint.config.js`](../../apps/website/frontend/eslint.config.js): **`eslint-config-prettier`** last (disable formatting rules; **no** `eslint-plugin-prettier`) |
| T6 | One-time format | Run `npm run format` on `src/**/*.{ts,tsx,css}` (+ `*.css` at frontend root); commit as formatting-only diff |
| T7 | CI wiring | [`Makefile`](../../Makefile) `ci-local-frontend`: add `format:check` after `npm ci`, before `lint`; add editorconfig step (repo root). [`ci.yml`](../../.github/workflows/ci.yml) frontend job: mirror |
| T8 | Shipped note | Append **Shipped (T-125.5):** under this section (only doc edit Claude may append) |

### Task 1 — `.editorconfig` (normative)

Minimum sections (extend as needed for monorepo paths):

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.go]
indent_style = tab

[*.{ts,tsx,js,mjs,cjs}]
indent_style = space
indent_size = 2

[*.{json,yml,yaml,md,css}]
indent_style = space
indent_size = 2
```

**Do not** override Go tab policy — FMT-1 `gofmt` is authoritative for Go formatting; editorconfig
only aligns editor defaults + checker.

### Task 2 — `editorconfig-checker` (FMT-2)

- Install/run: **`editorconfig-checker`** CLI (Go: `go install github.com/editorconfig-checker/editorconfig-checker/v3/cmd/editorconfig-checker@latest`, or pinned npm wrapper — pick one, document in Makefile comment).
- Run from **repo root** so `.editorconfig` applies to `apps/`, `packages/`, `docs/`, `scripts/`.
- **Exclude** (checker flags or `.editorconfig-checker.json` if needed): `node_modules/`, `dist/`,
  `apps/website/frontend/src/types/contract/**`, `apps/mod/**` binary/LFS paths, `.git/`.
- Fix any violations in the same commit (usually trailing whitespace / missing final newline).

### Task 3–5 — Prettier + eslint (FMT-3)

**Scope:** `apps/website/frontend/**/*.{ts,tsx,css}` only (not Go, not Enfusion `.c`, not generated
`src/types/contract/**`).

**Suggested `.prettierrc`** (align to existing code — verify against `src/lib/utils.ts`):

```json
{
  "semi": false,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "all",
  "printWidth": 100
}
```

**`.prettierignore`:** `dist`, `node_modules`, `src/types/contract`, `package-lock.json`.

**eslint:** add `eslint-config-prettier` and extend it **after** all other configs so TS-2..7/LOG-2/COMP-1
lint rules stay; only stylistic conflicts are turned off.

### Task 6 — One-time reformat

- Run `npm run format` once; expect a **large but formatting-only** diff across FE `src/`.
- **Do not** reformat `packages/tbd-schema` JSON fixtures or Go sources in this slice.
- If Prettier touches a line with an inline `eslint-disable` comment, verify `npm run lint` still passes.

### Task 7 — CI / local mirror

Update **`ci-local-frontend`** (after `npm ci`):

```makefile
npm run format:check    # FMT-3
npm run lint            # existing
npm run build && npm test
```

Add **editorconfig-checker** — either first step inside `ci-local-frontend` (shell `cd` to repo root)
or a dedicated `verify-editorconfig` Make target invoked from `ci-local` before backend. Must match
**`ci.yml` frontend job** (+ root-level editorconfig step if split).

**Out of scope:** registry/CLAUDE hub matrix status flip (T-125.6); CODING_STANDARDS.md body edits
(T-125.6); handler/route/error scripts (T-125.4 live); mod `.c` files.

**Verify (all exit 0):**

```bash
editorconfig-checker                          # FMT-2 (repo root)
cd apps/website/frontend && npm run format:check   # FMT-3
cd apps/website/frontend && npm run lint && npm run build && npm test
make ci-local                                 # full gate; report wall-clock
```

**Acceptance:** `make ci-local` green **and** `ci.yml` would pass on push. Formatting diff is
**style-only** (no logic changes).

---

## T-125.6 — Doc sync (Cursor)

- Mark T-125 **shipped** in registry; `./scripts/ticket sync`
- [`CLAUDE.md`](../../CLAUDE.md) §Done bullet
- Fix DOCUMENTATION_STANDARDS meta-drift (§0 “no codegen”; §10 eslint row)
- [`DEV_RUNBOOK.md`](../website/DEV_RUNBOOK.md) — CI replay commands

---

## Acceptance criteria (11/10)

- [ ] `CODING_STANDARDS.md` exists and cross-linked; distinct from DOCUMENTATION_STANDARDS
- [ ] **`ci.yml` green on `main`** — includes `make test-it`, FE build/lint/test, schema validate
- [ ] **golangci** runs full linter set **without** `only-new-issues`
- [x] **TypeScript `strict: true`** — build clean (T-125.3 @ `e5fbf4b`)
- [x] **Every handler** has `@route` matching `Register()`; GO-7 route-match live (T-125.4 @ `cb508cf`)
- [x] Citation verifier + coding-standards scripts exit 0 (T-125.4)
- [x] Replay commands documented in spec and DEV_RUNBOOK (T-125.4)

---

## Risk notes

- **Full gate on day one** produces a **large diff** (especially `strict: true` + errcheck + removing `only-new-issues`). Budget one heavy PR.
- **Postgres 18** in `ci.yml` must match T-124 compose image.
