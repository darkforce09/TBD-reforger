# T-125 — Coding standards + 11/10 enforcement

**Ticket:** T-125 · **Program:** platform · **Status:** **ready** (T-124 shipped @ `cd11db0`)  
**Depends on:** T-124 (met) · **Active slice:** T-125.4 · **Handoff:** [`.ai/artifacts/t125_claude_code_handoff.md`](../../.ai/artifacts/t125_claude_code_handoff.md)

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
| **T-125.5** | claude-code | `.editorconfig` / Prettier (if in standard) |
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
tags (GO-7), silent DB reads (M6 / GO-2), best-effort audit rationale (GO-3 where still bare),
and five new `scripts/website/verify-*` artefacts wired through **`make verify-coding-standards`**
and **`make ci-local`**. Optional: ENF-4 Enfusion DTO fixture branch in `validate.mjs`.

**Authority:** route wiring lives in [`handlers.go` `Register()`](../../apps/website/internal/handlers/handlers.go)
(all paths are under `/api/v1/…`). `@route` grammar: [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §3.1
(`@route GET /api/v1/missions`, include the `/api/v1` prefix).

### Task 1 — GO-7: `@route` on every HTTP handler

- **Scope:** every `func (h *Handler) <Name>(c *gin.Context)` in
  [`internal/handlers/*.go`](../../apps/website/internal/handlers/) **excluding** `*_test.go`.
  **Exclude** test helpers `JWT()`, `Discord()`, `Webhook()` (no `*gin.Context`).
- **Today:** **5 / ~90** tagged (`registry.go`, `missions.go` ×3, `field_tools.go` ×1).
- **Extend** [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs):
  second Go pass — preceding Godoc block on each `(h *Handler) … (c *gin.Context)` MUST contain
  `@route <METHOD> /api/v1/…` matching [`Register()`](../../apps/website/internal/handlers/handlers.go).
  Exit 1 with `file:line: HandlerName missing @route (GO-7)`.

### Task 2 — M6 / GO-2: fix silent `_ = h.db.First(…).Error`

**15 sites** (errcheck does not flag `.Error` field reads — deferred from T-125.2):

| File | Lines (approx.) | Fix pattern |
|------|-----------------|-------------|
| `dashboard.go` | 60, 94, 96, 98 | Check `.Error`; `errors.Is(…, gorm.ErrRecordNotFound)` → skip row or 404/500 |
| `deployments.go` | 66 | Same |
| `events.go` | 525 | Same |
| `cms.go` | 187, 192 | Same |
| `me.go` | 180 | Same |
| `wiki.go` | 84 | Same |
| `missions.go` | 294, 383, 515 | Same |
| `approvals.go` | 107, 143 | Same |

Use [`registry.go` `ListRegistry`](../../apps/website/internal/handlers/registry.go) as the reference
(correct `First` + 404/500 branching). **No blanket `//nolint`** on reads whose result is used.

### Task 3 — GO-3: bare `_ = services.WriteAudit(…)` 

Audit M6 also covers audit drops. Every discarded `WriteAudit` MUST carry
`//nolint:errcheck // best-effort: <why safe>` (GO-3). Grep `handlers/` for `_ = services.WriteAudit`
and annotate or handle (~15 sites across `admin.go`, `auth.go`, `cms.go`, `approvals.go`, `telemetry.go`,
`me.go`, `field_tools.go`).

### Task 4 — GO-1 / GO-9: `verify-handler-imports.sh`

New [`scripts/website/verify-handler-imports.sh`](../../scripts/website/verify-handler-imports.sh):

- Scan `apps/website/internal/handlers/*.go` excluding `*_test.go`.
- Each `"github.com/tbd-milsim/reforger-backend/internal/<pkg>"` import MUST be in allowlist:
  **`services`, `models`, `middleware`, `contract`, `config`** (+ stdlib / gin / gorm / uuid etc.).
- **Known violations today:** `handlers.go` imports **`auth`** + **`realtime`**; `telemetry.go` imports **`db`**.
  Prefer minimal relocation into `services/` without behaviour change; if too large for this slice, add
  **GO-9** rows to [`.coding-standards-allowlist.yaml`](../../.coding-standards-allowlist.yaml) with
  `expires` + split ticket ref and document in the Shipped note — do **not** delete the rule.

### Task 5 — ERR-4: `verify-error-envelope.sh`

New script: grep `c.JSON(http.Status4xx|5xx, gin.H{…})` (and `StatusBadRequest` etc.) in `handlers/`.
Assert top-level keys ⊆ **`{error, details}`** only (`message`, `err`, `status` as body keys fail).
Document false-positive carve-outs inline in the script if needed (should be rare).

### Task 6 — LOG-3: `verify-handler-logging.sh`

New script: heuristic gate — handlers that return **4xx/5xx of consequence** (not bare expected 401/404
lookups) should log identifier + status + duration (mirror `CreateVersion` `log.Printf` pattern).
Start conservative (warn-only band acceptable in v1 if zero false positives is hard); prefer
**error exit** once the heuristic is stable. Document exemptions in script comments.

### Task 7 — SIZE-1 / SIZE-3: `verify-file-length.mjs`

New [`scripts/website/verify-file-length.mjs`](../../scripts/website/verify-file-length.mjs):

- **>600 lines** → WARN to stderr (SIZE-1, exit 0).
- **>1000 lines** → exit 1 (SIZE-3) unless path matches [`.coding-standards-allowlist.yaml`](../../.coding-standards-allowlist.yaml).
- **Standing debt** (add SIZE-3 allowlist rows if not present):

  | File | Lines | Split plan |
  |------|------:|------------|
  | `apps/website/frontend/src/pages/admin.tsx` | ~1628 | admin sub-surfaces |
  | `apps/website/frontend/src/pages/doctrine.tsx` | ~1288 | wiki split-pane helpers |
  | `apps/website/internal/handlers/events.go` | ~1038 | ORBAT → `services/` (GO-1) |

- SIZE-2 MC allowlist (`tactical-map/**`) already in YAML — honour it.

### Task 8 — ENF-4: Enfusion DTO fixture gate

Extend [`packages/tbd-schema/scripts/validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs):

- For each Enfusion `.c` DTO under `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` (start with
  [`TBD_MissionSlotStruct.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c)
  — already `@contract mission.schema.json#/$defs/slot`).
- Require a golden JSON fixture (e.g. `packages/tbd-schema/enfusion/mission-slot.sample.json`) that
  validates against the cited schema pointer.
- **Do not edit** other mod `.c` files unless required for ENF-4; consult `enfusion-mcp` if touching Enfusion.

### Task 9 — Wire `make verify-coding-standards` + `ci-local`

In root [`Makefile`](../../Makefile):

```makefile
verify-coding-standards: ## GO-1/9, ERR-4, LOG-3, SIZE-1/3 script bundle
	bash scripts/website/verify-handler-imports.sh
	bash scripts/website/verify-error-envelope.sh
	bash scripts/website/verify-handler-logging.sh
	node scripts/website/verify-file-length.mjs
```

Add **`$(MAKE) verify-coding-standards`** to **`ci-local`** (after backend tests or before schema job —
document choice in Shipped note). Optionally add GO-7 `@route` pass to `ci-local-schema` via existing
`verify-citations` (preferred — one script).

Note: **`@model` on `types/api/index.ts`** was completed in **T-125.3** (TS-6 gate required it).

**Verify:** `make verify-citations` (incl. new `@route` pass) · `make verify-coding-standards` ·
`make test-it` · `make ci-local` all exit 0.

**Out of scope:** Prettier/editorconfig (T-125.5); registry/CLAUDE/CODING_STANDARDS matrix status sync
(T-125.6); refactoring `events.go` / page god-components beyond M6 fixes; TS/eslint (T-125.3 live).

---

## T-125.5 — Repo hygiene

- Root **`.editorconfig`**
- Optional **Prettier** + `format` script (if approved in CODING_STANDARDS)

**Verify:** formatting consistent; no CI regression.

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
- [ ] **Every handler** has `@route` in Godoc; cross-boundary TS types have `@model`/`@contract` where applicable (TS-6 live; GO-7 → T-125.4)
- [ ] Citation verifier + any new tag verifiers exit 0
- [ ] Replay commands documented in spec and DEV_RUNBOOK

---

## Risk notes

- **Full gate on day one** produces a **large diff** (especially `strict: true` + errcheck + removing `only-new-issues`). Budget one heavy PR.
- **Postgres 18** in `ci.yml` must match T-124 compose image.
