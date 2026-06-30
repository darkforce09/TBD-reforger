# TBD Reforger — Coding Standards

**Status:** living
**Audience:** every engineer and AI agent that writes Go, TypeScript/React, or Enfusion code in this monorepo
**Authority:** Running code → [`CLAUDE.md`](../../CLAUDE.md) → [`docs/platform/README.md`](README.md) → **this doc** (supporting tier)
**Updated:** 2026-06-30
**Ticket:** [T-125](t125_coding_standards_enforcement.md) — **shipped** @ `e21dac3` (tag **T-125.5**); program **T-125.0–.6 complete** (38 rules, all CI gates live).

> This document is the source of truth for **how code is written** across the three boundaries of
> `TBD-Reforger`. Its sibling, [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md), owns **how
> code is documented** (the cross-boundary tag vocabulary and per-language doc-comment rules) and
> **where markdown files live** (§8.2). The two do not overlap — see the boundary matrix in §0.1. This doc is **prescriptive**: **MUST**/**SHALL**
> are mandatory, **FORBIDDEN** patterns must not be introduced, and every rule names exactly one
> enforcement **gate** (§0.2). It defers to running code and never overrides a rule in
> [`CLAUDE.md`](../../CLAUDE.md) or the [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md).

---

## 0. Why this exists & the documentation boundary

The repo documents its *contracts* well (DOCUMENTATION_STANDARDS.md, the `@contract`/`@route` CI
gates). What it lacked was a written, **enforced** standard for the **code itself** — when a handler
is too fat, whether a swallowed `_ = db.First(...)` is acceptable, what HTTP status a duplicate key
returns, how big a file may grow, and — critically — **which tool checks each rule**. The 2026 audit
([`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md)) surfaced the symptoms: **M6** (31 swallowed
DB/audit errors), god-files (`admin.tsx` 1628 L, `doctrine.tsx` 1289 L, `events.go` 1041 L), and
inconsistent error envelopes. This document fixes that. Go lint is **gated by the full
[`apps/website/.golangci.yml`](../../apps/website/.golangci.yml) set** (revive, errcheck, errorlint,
staticcheck, govet, cyclop) on every **`ci.yml`** push/PR to `main` and via **`make ci-local`**.
[`contracts.yml`](../../.github/workflows/contracts.yml) is a path-filtered supplement (no
`only-new-issues` since **T-125.2**). §10 maps **every rule to the exact tool, config, verify command,
and T-125 slice** that enforces it.

### 0.1 Boundary matrix — what lives where (zero overlap)

| Concern | Owner | Notes |
|---------|-------|-------|
| `@contract` / `@route` / `@model` / `@consumer` tag grammar | **DOCUMENTATION_STANDARDS.md** §3 | Do **not** restate grammar here. |
| Godoc / TSDoc / Doxygen **doc-comment** rules (presence, prose style) | **DOCUMENTATION_STANDARDS.md** §4–§6 | Code rules here point up to it. |
| Enfusion network-authority **tags** (`@authority`/`@rpc`/`@replicated`) | **DOCUMENTATION_STANDARDS.md** §7 | This doc covers Enfusion *code* policy (logging, gates), not the tags. |
| Schema-as-source-of-truth, codegen, runtime validation | **DOCUMENTATION_STANDARDS.md** §2, §9 | — |
| Architectural decision records (ADR tiers) | **DOCUMENTATION_STANDARDS.md** §8 | — |
| **Code structure** (handler vs `services/`, `pages/` vs `features/`) | **this doc** §2–§3 | — |
| **Error handling** (`_ =` policy, `%w` wrapping, error envelope, status codes) | **this doc** §2, §4 | — |
| **Testing bar** per layer | **this doc** §6 | — |
| **Formatting** (gofmt, `.editorconfig`, Prettier) | **this doc** §7 | — |
| **File size / complexity** limits + MC allowlist | **this doc** §8 | — |
| **Logging** policy (Go, FE, Enfusion) | **this doc** §9 | — |
| **Code** CI gates (golangci full set, `tsc strict`, `ci.yml`) | **this doc** §10 | Doc/citation gates stay in DOCUMENTATION_STANDARDS.md §10. |

Rule of thumb: **a *comment/tag* rule lives in DOCUMENTATION_STANDARDS.md; a *code* rule lives here.**
Where a code rule depends on a doc rule (e.g. exported-symbol Godoc), this doc **points** to the
other and does not duplicate the text.

### 0.2 Gate taxonomy — every §10 rule uses exactly ONE

| Gate | Meaning | CI behavior |
|------|---------|-------------|
| **CI-BLOCK** | A tool exits non-zero on violation. | Required job in [`ci.yml`](../../.github/workflows) / [`contracts.yml`](../../.github/workflows/contracts.yml). |
| **CI-SCRIPT** | A repo script exits non-zero on violation. | `node packages/tbd-schema/scripts/…` or `scripts/website/verify-*.sh`, run by `make ci-local`. |
| **ALLOWLIST** | A CI-SCRIPT plus a checked-in allowlist file. | Reads `.coding-standards-allowlist.yaml` (§8.1); an unlisted violation exits non-zero. |
| **MANUAL** | No static automation is possible (Enfusion runtime / Workbench only). | MUST cite why; **maximum 3** MANUAL rules repo-wide; **FORBIDDEN** for any Go/TS/API rule once T-125.5 ships. |

Normative verbs are **SHALL / MUST / FORBIDDEN**. Vague qualifiers and percentage hand-waves are
**FORBIDDEN** in this document: state an exact number, command, or tool — a rule that cannot be stated
precisely is not ready to ship.

### 0.3 Meta-gates — rules about the CI configuration itself

- **CI-1 (Debuggability) — `only-new-issues` SHALL NOT survive.** After **T-125.2** (shipped),
  [`contracts.yml`](../../.github/workflows/contracts.yml) MUST NOT set `only-new-issues: true` on the
  golangci job. Gate: **CI-SCRIPT** — [`scripts/website/verify-ci1.sh`](../../scripts/website/verify-ci1.sh),
  run inside `make ci-local-backend`.
- **CI-2 (Debuggability) — `ci.yml` SHALL gate every push/PR to `main`.** It MUST run **backend**
  (Postgres 18, `make test-it`), **frontend** (`npm run lint`/`build`/`test`), and **schema**
  (`make schema-validate`, `make verify-citations`). Gate: **CI-BLOCK** (the workflow itself). Slice **T-125.1**.

---

## 1. The four pillars

Every rule serves one primary pillar — the *why*. The rule is the *what*; §10 is the *how it's checked*.

| Pillar | The question it answers | Example rules |
|--------|-------------------------|---------------|
| **Scalability** | Workable at 10× size / data / team? | logic in `services/` (GO-1, GO-9), `pages/` layering (TS-2), file-size gate (SIZE-1/3), MC allowlist (SIZE-2) |
| **Readability** | Understandable without archaeology? | Godoc/TSDoc + tags (GO-6/7, TS-5/6, ENF-3), gofmt/editorconfig/Prettier (FMT-1–3), complexity cap (COMP-1) |
| **Usability** | Correct, predictable contract for the consumer? | error envelope + status table (ERR-1/2/4/5), duplicate-key 409 (GO-5), surfaced FE errors (TS-4/7), DTO fixtures (ENF-4) |
| **Debuggability** | At 02:00, can we tell *what* and *why* fast? | handled DB errors + `%w` (GO-2/3/4/8), `strict` (TS-1/3), structured logs (LOG-2/3), tests (TEST-1–3), CI gates (CI-1/2) |

---

## 2. Go

The backend is Gin + GORM. Handlers are the HTTP edge; `internal/services/` is the logic core;
`internal/models/` is the snake_case DB/API contract.

**REQUIRED**

- **GO-1 (Scalability) — Business logic SHALL live in `services/`; handlers do HTTP only.** A handler
  in [`internal/handlers/`](../../apps/website/internal/handlers) binds/validates input, checks authz,
  calls a service, and maps the result to a status + body. Multi-step DB work, ORBAT materialisation,
  and telemetry math live in [`internal/services/`](../../apps/website/internal/services). Gate:
  **CI-SCRIPT** (`verify-handler-imports.sh`, §10). `events.go` (1041 L) is the standing
  counter-example (§8).
- **GO-2 (Debuggability) — DB-read errors MUST be handled; no silent `_ =` on a query whose result is
  used.** A `_ = h.db.First(&x, …)` that then reads `x` hides "row not found"/connection errors. Check
  `.Error` and branch (404 / 500). Gate: **CI-BLOCK** (errcheck `check-blank`) **plus** the M6 handler
  fixes shipped in **T-125.4** (15 sites; enrichment paths log non-`NotFound` even at 200).
- **GO-3 (Debuggability) — A best-effort write MUST carry a rationale.** Discarding an error is allowed
  **only** with `//nolint:errcheck // best-effort: <why dropping is safe>` on the line (most
  `services.WriteAudit(...)`). A bare `_ = WriteAudit(...)` is a defect. Gate: **CI-BLOCK** (errcheck
  `check-blank: true` flags the unannotated blank-assign).
- **GO-4 (Debuggability) — Propagated errors MUST wrap the cause with `%w`.** Use
  `fmt.Errorf("create version: %w", err)`. Gate: **CI-BLOCK** (`errorlint`).
- **GO-5 (Usability) — A unique-constraint clash MUST return 409 via SQLSTATE `23505`, not a string
  match.** Detect `*pgconn.PgError` code `23505` (not `strings.Contains(err.Error(), "duplicate")`,
  audit T6/M6). Gate: **CI-BLOCK** (integration test `TestDuplicateSemver_409` + `staticcheck`).
- **GO-6 (Readability) — Every exported identifier MUST carry a Godoc comment starting with its name.**
  Owned by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §4. Gate: **CI-BLOCK** (golangci
  `revive` `exported`); **T-125.2** removes `only-new-issues`, making it a full-repo gate.
- **GO-7 (Readability) — Every exported handler func SHALL carry `@route` in its Godoc, and the tag
  MUST match the wired route in `handlers.go` `Register()` (method + path).** The three-way
  triangulation of DOCUMENTATION_STANDARDS.md §3. Gate: **CI-SCRIPT** —
  `verify-contract-citations.mjs` GO-7 pass (presence **and** Register route-match on all 82 handlers).
- **GO-8 (Debuggability) — `staticcheck` (all checks) SHALL be enabled.** Generated
  `internal/contract/**` is excluded via `issues.exclude-rules`. Gate: **CI-BLOCK** (`.golangci.yml`).
- **GO-9 (Scalability) — The `handlers` package SHALL import only `services`, `models`, `middleware`,
  `contract`, `config` (+ std/gin).** It MUST NOT reach into other application packages for logic
  reuse — that belongs in a service. Structural imports of `internal/auth` and `internal/realtime` on
  `handlers.go`, `auth.go`, and `me.go` are allowlisted in `.coding-standards-allowlist.yaml`
  (`expires: structural`). Gate: **CI-SCRIPT** (`verify-handler-imports.sh` import allowlist).

**FORBIDDEN**

- Business logic / raw multi-table SQL inline in a handler when a service would carry it (GO-1/GO-9).
- A blank-assigned (`_ =`) DB/exec error with no `//nolint` rationale (GO-2/GO-3).
- `panic` / `log.Fatal` on a request path.

---

## 3. TypeScript / React

Vite + React 19 + TanStack Query + Zustand. `src/types/` is the hand-written API contract mirror;
`src/api/` the axios layer; `src/hooks/` the query/mutation layer; `src/pages/` route screens;
`src/features/` self-contained domains; `src/components/ui/` shared primitives.

**REQUIRED**

- **TS-1 (Debuggability) — `tsconfig.app.json` + `tsconfig.node.json` `compilerOptions.strict` MUST be
  `true`.** Live @ **T-125.3** (`npm run build` = `tsc -b` builds both). Gate: **CI-BLOCK**
  (`tsc -b` via `npm run build`).
- **TS-2 (Scalability) — Layer boundaries SHALL hold.** `pages/` compose a route from hooks +
  feature/`ui` components and own *data wiring* only; reusable logic and heavy surfaces live in
  `features/`; cross-page primitives in `components/ui/`. A `page` MUST NOT be imported by a `feature`
  or `component`. Gate: **CI-BLOCK** (eslint `import-x/no-restricted-paths` zones + built-in
  `no-restricted-imports` for the `@/pages` alias — `eslint-plugin-import` peers eslint ≤9).
- **TS-3 (Debuggability) — No `any`; no unsafe non-null `!` on contract data.** Gate: **CI-BLOCK**
  (eslint `@typescript-eslint/no-explicit-any` + `no-non-null-assertion`).
- **TS-4 (Usability) — A failed query/mutation MUST surface a user-visible error state.** Mirror
  `useMissionEditor.saveVersion` (413 → "too large", 409 → semver, else backend `error`). The
  enforceable invariant is TS-7 (no swallowing catch). Gate: **CI-BLOCK** (eslint `no-empty`).
- **TS-5 (Readability) — Contract-layer exports (`types/`, `api/`, `hooks/`) MUST carry a TSDoc block
  (presence).** Owned by [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §5. Gate:
  **CI-BLOCK** (eslint-plugin-jsdoc `require-jsdoc`, live in [`eslint.config.js`](../../apps/website/frontend/eslint.config.js)).
- **TS-6 (Readability) — Cross-boundary exports MUST include `@contract` or `@model` content (not just
  a block).** Gate: **CI-SCRIPT** — `verify-contract-citations.mjs` requires the tag on exported
  `interface`/`type` in `types/`, `api/`, `hooks/` (live @ **T-125.3**; generic envelopes like
  `Paginated<T>` exempt).
- **TS-7 (Usability) — Empty or log-only `catch` blocks are FORBIDDEN.** A catch must surface,
  re-throw, or recover. Gate: **CI-BLOCK** (eslint `no-empty {allowEmptyCatch:false}` + `no-empty-function`).

**FORBIDDEN**

- `any` (explicit or via strict-off) on wire/contract types (TS-1/TS-3).
- A `page` imported by a `feature`/`component`, or business logic in a `page` (TS-2).
- A `catch` that neither surfaces nor re-throws (TS-4/TS-7).

---

## 4. Errors & the HTTP contract

The API speaks **one** error shape. This section is normative for every JSON handler.

- **ERR-1 (Usability) — The error envelope is `{ "error": string }`** (+ optional `"details":
  string[]` for validation). Reference: `CreateVersion` →
  `{ "error": "invalid mission payload", "details": [...] }`. Gate: **CI-BLOCK** (integration tests
  assert the body shape on 400/404/409/413 fixtures). *Success* lists stay `{ data, total, limit,
  offset }` ([`CLAUDE.md`](../../CLAUDE.md) §Conventions); audit logs use `next_cursor`.
- **ERR-2 (Usability) — Status codes MUST follow the table:**

  | Status | Meaning | Used when |
  |--------|---------|-----------|
  | `200 OK` | success (read/update) | normal GET/PATCH |
  | `201 Created` | resource created | POST that persists (mission, version) |
  | `400 Bad Request` | malformed/invalid input | bind failure, schema-invalid payload (`details[]`) |
  | `401 Unauthorized` | no/invalid auth | missing or bad JWT |
  | `403 Forbidden` | authn ok, authz denied | wrong role, "not your mission" |
  | `404 Not Found` | resource absent | unknown id, draft hidden from non-author |
  | `409 Conflict` | state/uniqueness clash | duplicate semver, unique-key `23505` (GO-5) |
  | `413 Payload Too Large` | body over the route cap | mission version past `MissionVersionBodyLimit` |
  | `500 Internal Server Error` | unexpected server fault | unhandled DB/internal error |

  Gate: **CI-BLOCK** (integration status-matrix subtests).
- **ERR-4 (Usability) — No error body MAY carry a top-level key outside `{error, details}`.** Gate:
  **CI-SCRIPT** — `verify-error-envelope.sh` (awk brace-balanced scan of every
  `c.JSON(http.Status*, gin.H{…})`; keys `message`/`err`/`errors`/`status` fail the build). Live @
  **T-125.4** — caught + fixed `field_tools.go` 422 `solution`→`details`.
- **ERR-5 (Usability) — Each status class SHALL have one named integration subtest per resource.**
  e.g. `TestCreateVersion_InvalidPayload_400`, `TestDuplicateSemver_409`, `TestMission_NotFound_404`,
  `TestVersion_TooLarge_413`. Gate: **CI-BLOCK** (`make test-it`).

*The "log on 4xx/5xx" requirement formerly drafted as ERR-3 is consolidated into **LOG-3** (§9).*

---

## 5. Enfusion / Enforce Script — code policy

This section covers Enfusion **code** behaviour. The networked-code **tags**
(`@authority`/`@rpc`/`@replicated`/`@contract`) and doc-comment rules are **owned by**
[`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §6–§7 — not restated here.

- **ENF-1 (Debuggability) — Disciplined logging; dev toggles ship OFF.** Use `Print(..., LogLevel.X)`
  with a level; no per-frame / per-replication-tick logging on hot paths. Any developer test switch
  defaults to `[Attribute("0")]` (audit C4 `TBD_LoadoutEquipComponent`, T16 RegistryPoc). Gate:
  **MANUAL** — Enfusion log levels and `[Attribute]` defaults are validated at Workbench runtime;
  there is no static analyser for Enforce Script in this repo.
- **ENF-2 (Debuggability) — Authority gates are annotated.** Every `if (RplSession.Mode() ==
  RplMode.Client) return;` carries a `// Authority only — <reason>` line. Gate: **MANUAL** (same
  Enforce-Script no-static-analyser reason as ENF-1).
- **ENF-3 (Readability) — Networked-code tags MUST resolve.** `@contract`/`@authority` (per
  DOCUMENTATION_STANDARDS.md §6–§7) on `.c` files. Gate: **CI-SCRIPT** (`verify-contract-citations.mjs`).
- **ENF-4 (Usability) — Every JSON-parsed DTO MUST have a golden fixture that validates.** Gate:
  **CI-SCRIPT** — the Enfusion DTO branch of [`validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs)
  (10 Backend `@contract` DTOs → `packages/tbd-schema/enfusion/*.sample.json`; live @ **T-125.4**).

**Process (from [`CLAUDE.md`](../../CLAUDE.md)):** do **not** edit `apps/mod` `.c` files unless a ticket
slice explicitly assigns `claude-code` to that path, and **use `enfusion-mcp` before editing any `.c`
file**. `make test-it` / the FE build do **not** cover Enfusion; mod changes need a Workbench pass —
this is precisely why ENF-1/ENF-2 are the only sanctioned **MANUAL** gates.

---

## 6. Testing — the minimum bar per layer

- **TEST-1 (Debuggability) — A handler behaviour change MUST ship a green `make test-it`.**
  Integration tests in `internal/handlers/*_integration_test.go` run against a live Postgres
  (`make db-up`). Compilation (`go build`) is **not** proof of contract. Gate: **CI-BLOCK** (ci.yml backend).
- **TEST-2 (Debuggability) — Non-trivial `features/` hooks & pure utils MUST have a `vitest` test.**
  Compilers, selectors, transforms. `npm test` → `vitest run` (vitest 4.1.9). Gate: **CI-BLOCK** (ci.yml frontend).
- **TEST-3 (Usability) — A schema/DTO change MUST ship a golden fixture + green `make schema-validate`.**
  Gate: **CI-BLOCK** (ci.yml schema job → `scripts/validate.mjs`).

---

## 7. Formatting & hygiene

- **FMT-1 (Readability) — Go is gofmt clean.** Gate: **CI-BLOCK** —
  `test -z "$(gofmt -l apps/website/internal apps/website/cmd)"`.
- **FMT-2 (Readability) — A root `.editorconfig` governs whitespace** (UTF-8, LF, final newline,
  trailing-whitespace trim; tabs for Go, 2-space for TS/JSON/YAML). Live @ **T-125.5** (`e21dac3`).
  Gate: **CI-BLOCK** (`editorconfig-checker` via `make verify-editorconfig`).
- **FMT-3 (Readability) — Prettier is the TS/TSX/CSS formatter-of-record.** eslint keeps lint rules,
  drops formatting opinions via `eslint-config-prettier`; `format` + `format:check` npm scripts.
  Live @ **T-125.5** (one-time repo-wide reformat diff). Gate: **CI-BLOCK** (`npm run format:check`).

---

## 8. File size & cyclomatic complexity

- **SIZE-1 (Scalability) — Files over **600 lines** emit a WARN.** Advisory tier of the file-length
  script; it does not fail the build but flags the file for a split. Gate: **CI-SCRIPT**
  (`verify-file-length.mjs`, warn band).
- **SIZE-2 (Scalability) — Mission Creator hot paths are allowlisted.** Files under
  [`src/features/tactical-map/**`](../../apps/website/frontend/src/features/tactical-map) are exempt
  from SIZE-3 — deliberately dense performance code from the **T-057..T-067** scale program
  (`state/ydoc.ts` 756 L, `state/slotIconCache.ts`, `tools/useSelectTool.ts`) where measured fps
  outweighs file length. Gate: **ALLOWLIST** (`.coding-standards-allowlist.yaml`, `reason: MC-perf`).
- **SIZE-3 (Scalability) — Files over **1000 lines** fail the build unless allowlisted.** Gate:
  **CI-SCRIPT** (`verify-file-length.mjs` → exit 1). Standing debt carries an allowlist entry with an
  `expires` date until its split ticket lands:

  | File | Lines | Split plan |
  |------|------:|------------|
  | `pages/admin.tsx` | 1628 | split by admin sub-surface (Personnel / Approvals / Audit) |
  | `pages/doctrine.tsx` | 1289 | extract the wiki split-pane helpers |
  | `handlers/events.go` | 1041 | extract ORBAT + registration into `services/` (GO-1) |

- **COMP-1 (Readability) — Cyclomatic complexity ≤ 15 per function (hard gate).** A function over 15
  independent paths is split into named helpers. Gate: **CI-BLOCK** — Go via golangci **`cyclop`**
  (`max-complexity: 15`), TypeScript via ESLint `complexity: ["error", { max: 15 }]`. The **only**
  escape is a *per-function* inline opt-out with a rationale (the GO-3 pattern):
  - Go: `//nolint:cyclop // <why this function must branch this much>`
  - TS: `// eslint-disable-next-line complexity -- <why>`

  This applies to `features/tactical-map/**` as well — the SIZE-2 file-size allowlist does **not**
  extend to complexity. A dense fps hot-path function takes the inline opt-out so the exception is
  named and auditable.

### 8.1 Allowlist contract — `.coding-standards-allowlist.yaml`

Created in **T-125.2** at the repo root. Each entry is normative:

```yaml
- rule: SIZE-3            # the Rule ID being excepted
  path: apps/website/internal/handlers/events.go
  symbol:                 # OPTIONAL — function/type for fn-level rules
  reason: pre-existing god-file; split tracked by T-1xx
  expires: 2026-09-30     # YYYY-MM-DD, or "MC-perf" for permanent hot-path exemptions
```

**Opt-out policy (one policy, no ambiguity):**
- **Function-level** opt-outs (**COMP-1**, **GO-3**) live **inline** (`//nolint` / `eslint-disable`
  with a reason) — never in the allowlist file.
- **File-level** opt-outs (**SIZE-2**, **SIZE-3** named-debt + MC paths) live **only** in
  `.coding-standards-allowlist.yaml` with a `reason` and `expires`. A CI-SCRIPT FORBIDS an expired entry.

---

## 9. Logging

- **LOG-2 (Debuggability) — No committed FE `console.log`.** Dev HUDs/counters (`FpsCounter`, audit
  T12) sit behind a dev/env guard; `console.error`/`console.warn` for real errors is allowed. Gate:
  **CI-BLOCK** (eslint `no-console {allow:["warn","error"]}`).
- **LOG-3 (Debuggability) — A handler 4xx/5xx of consequence MUST log path + status + duration**
  (the `logHandlerErr` helper + `middleware.Timing()` pattern; `c.FullPath()` not `c.Param("id")`).
  **Band 1:** every **5xx** (75 sites). **Band 2:** mutator **400/409/413** on POST/PUT/PATCH/DELETE
  (65 sites in T-125.4 ship). Operational side-effects that still return 200 (e.g. failed
  `RefreshLeaderboard` after ingest) MUST log anyway. Expected misses (bare GET **404**, auth **401**)
  are exempt. Gate: **CI-SCRIPT** (`verify-handler-logging.sh` — POSIX awk + Register-derived mutator set).

*This consolidates the former LOG-1 (structured logs) and the §4 ERR-3 draft into one enforced rule.*

---

## 10. Enforcement matrix

Every rule, its gate (§0.2), the exact tool + config, the local verify command (exit 0 = pass), the
slice that wires it, and whether it is **live** today or **planned**. **Pillar:** Sc=Scalability,
Re=Readability, Us=Usability, De=Debuggability.

| Rule | Pillar | Statement | Gate | Enforcement (tool + config) | Verify (exit 0) | Slice | Status |
|------|:--:|-----------|------|-----------------------------|-----------------|:--:|:--:|
| **GO-1** | Sc | Logic in `services/`; handlers HTTP-only | CI-SCRIPT | `scripts/website/verify-handler-imports.sh` | `bash scripts/website/verify-handler-imports.sh` | T-125.4 | live |
| **GO-2** | De | Handle DB-read errors (no silent `_=`) | CI-BLOCK | golangci `errcheck` (`check-blank: true`) + M6 fixes | `cd apps/website && golangci-lint run ./...` | T-125.2/.4 | live |
| **GO-3** | De | Best-effort write needs `//nolint:errcheck // best-effort:` | CI-BLOCK | golangci `errcheck` `check-blank: true` | `golangci-lint run ./...` | T-125.2 | live |
| **GO-4** | De | Wrap propagated errors with `%w` | CI-BLOCK | golangci `errorlint` | `golangci-lint run ./...` | T-125.2 | live |
| **GO-5** | Us | Dup key → 409 via SQLSTATE `23505` | CI-BLOCK | IT `TestDuplicateSemver_409` + `staticcheck` | `make test-it` | T-125.4 | live |
| **GO-6** | Re | Exported Godoc starts with name | CI-BLOCK | golangci `revive` `exported` (no `only-new-issues`) | `golangci-lint run ./...` | T-125.2 | live |
| **GO-7** | Re | Handler `@route` matches `Register()` route | CI-SCRIPT | `verify-contract-citations.mjs` GO-7 pass (82 handlers) | `make verify-citations` | T-125.4 | live |
| **GO-8** | De | `staticcheck` on; `internal/contract/**` excluded | CI-BLOCK | `.golangci.yml`: `staticcheck` + `linters.exclusions.rules` path | `golangci-lint run ./...` | T-125.2 | live |
| **GO-9** | Sc | `handlers` imports ⊆ allowed + structural allowlist | CI-SCRIPT | `scripts/website/verify-handler-imports.sh` (import allowlist) | `bash …/verify-handler-imports.sh` | T-125.4 | live |
| **TS-1** | De | `tsconfig.*.json` `strict:true` (`tsc -b`) | CI-BLOCK | `tsc -b` | `npm run build` | T-125.3 | live |
| **TS-2** | Sc | `pages/` wiring-only; no page imported by feature | CI-BLOCK | eslint `import-x/no-restricted-paths` + `no-restricted-imports` (`@/pages`) | `npm run lint` | T-125.3 | live |
| **TS-3** | De | No `any` / unsafe `!` on contract data | CI-BLOCK | eslint `no-explicit-any` + `no-non-null-assertion` | `npm run lint` | T-125.3 | live |
| **TS-4** | Us | API errors surfaced to user | CI-BLOCK | eslint `no-empty {allowEmptyCatch:false}` (mech = TS-7) | `npm run lint` | T-125.3 | live |
| **TS-5** | Re | Contract-layer export has TSDoc block | CI-BLOCK | `eslint-plugin-jsdoc` `require-jsdoc` | `npm run lint` | — | live |
| **TS-6** | Re | Cross-boundary export has `@contract`/`@model` | CI-SCRIPT | `verify-contract-citations.mjs` (tag-content) | `make verify-citations` | T-125.3 | live |
| **TS-7** | Us | Empty/log-only `catch` FORBIDDEN | CI-BLOCK | eslint `no-empty` + `no-empty-function` | `npm run lint` | T-125.3 | live |
| **ERR-1** | Us | Body = `{error}` (+`details[]`) | CI-BLOCK | IT body-shape asserts on 400/404/409/413 | `make test-it` | T-125.4 | planned |
| **ERR-2** | Us | Status codes per §4 table | CI-BLOCK | IT status-matrix subtests | `make test-it` | T-125.4 | planned |
| **ERR-4** | Us | No error key outside `{error,details}` | CI-SCRIPT | `scripts/website/verify-error-envelope.sh` (awk brace-balanced) | `bash …/verify-error-envelope.sh` | T-125.4 | live |
| **ERR-5** | Us | One named IT per status class per resource | CI-BLOCK | `make test-it` (`Test*_400/404/409/413`) | `make test-it` | T-125.4 | planned |
| **ENF-1** | De | Log policy; dev toggles default off | MANUAL | Enfusion runtime — no Enforce-Script static analyser | Workbench pass | T-125.4 | manual |
| **ENF-2** | De | `// Authority only — <reason>` on gates | MANUAL | Enfusion runtime — no Enforce-Script static analyser | Workbench pass | — | manual |
| **ENF-3** | Re | `@contract`/`@authority` resolve on `.c` | CI-SCRIPT | `verify-contract-citations.mjs` | `make verify-citations` | T-125.4 | live |
| **ENF-4** | Us | DTO has validating golden fixture | CI-SCRIPT | `validate.mjs` Enfusion DTO branch (10 fixtures) | `make schema-validate` | T-125.4 | live |
| **TEST-1** | De | Handler change ⇒ `make test-it` green | CI-BLOCK | `ci.yml` backend (PG18) | `make test-it` | T-125.1 | live |
| **TEST-2** | De | features hooks/utils ⇒ vitest | CI-BLOCK | `ci.yml` frontend | `npm test` | T-125.1 | live |
| **TEST-3** | Us | Schema change ⇒ validate + fixture | CI-BLOCK | `ci.yml` schema | `make schema-validate` | T-125.1 | live |
| **FMT-1** | Re | gofmt clean | CI-BLOCK | `gofmt -l` empty | `test -z "$(gofmt -l apps/website/internal apps/website/cmd)"` | T-125.1 | live |
| **FMT-2** | Re | `.editorconfig` honored | CI-BLOCK | `editorconfig-checker` | `make verify-editorconfig` | T-125.5 | live |
| **FMT-3** | Re | Prettier for TS/TSX/CSS | CI-BLOCK | `prettier --check` | `npm run format:check` | T-125.5 | live |
| **SIZE-1** | Sc | >600 L ⇒ WARN | CI-SCRIPT | `verify-file-length.mjs` (warn band) | `node scripts/website/verify-file-length.mjs` | T-125.4 | live |
| **SIZE-2** | Sc | `tactical-map/**` size-exempt | ALLOWLIST | `.coding-standards-allowlist.yaml` (`reason: MC-perf`) | `node scripts/website/verify-file-length.mjs` | T-125.2 | live |
| **SIZE-3** | Sc | >1000 L ⇒ exit 1 unless allowlisted | CI-SCRIPT | `scripts/website/verify-file-length.mjs` | `node scripts/website/verify-file-length.mjs` | T-125.4 | live |
| **COMP-1** | Re | Cyclomatic ≤ 15/fn (hard); inline opt-out only | CI-BLOCK | golangci `cyclop` `max-complexity:15` · eslint `complexity:["error",{max:15}]` | `golangci-lint run ./...` · `npm run lint` | T-125.2/.3 | live |
| **LOG-2** | De | No committed FE `console.log` | CI-BLOCK | eslint `no-console {allow:["warn","error"]}` | `npm run lint` | T-125.3 | live |
| **LOG-3** | De | 5xx + mutator 4xx log path+status+dur | CI-SCRIPT | `scripts/website/verify-handler-logging.sh` | `bash …/verify-handler-logging.sh` | T-125.4 | live |
| **CI-1** | De | No `only-new-issues:true` post-T-125.2 | CI-SCRIPT | `scripts/website/verify-ci1.sh` (via `make ci-local-backend`) | `bash scripts/website/verify-ci1.sh` | T-125.2 | live |
| **CI-2** | De | `ci.yml` gates every push/PR to main | CI-BLOCK | `ci.yml` backend+frontend+schema jobs | `make ci-local` (mirror) | T-125.1 | live |

**Count by pillar:** Scalability 6 · Readability 9 · Usability 9 · Debuggability 14 · **38 total.**
**Count by gate:** CI-BLOCK 24 · CI-SCRIPT 11 · ALLOWLIST 1 · MANUAL 2 (ENF-1, ENF-2 — Enfusion only).

### 10.1 CI scripts inventory

Enforcement artefacts in the repo (T-125.1–.4). Primary workflow:
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml); local mirror **`make ci-local`**
(CODING_STANDARDS §11).

| Script / artefact | Rules it satisfies | Slice | Status |
|-------------------|--------------------|:-----:|:------:|
| [`verify-ci1.sh`](../../scripts/website/verify-ci1.sh) | CI-1 | T-125.2 | live |
| [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs) — TS-6 FE `@model`/`@contract` | TS-6 | T-125.3 | live |
| [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs) — GO-7 `@route` route-match (82 handlers) | GO-7, ENF-3 | T-125.4 | live |
| [`verify-handler-imports.sh`](../../scripts/website/verify-handler-imports.sh) | GO-1, GO-9 | T-125.4 | live |
| [`verify-error-envelope.sh`](../../scripts/website/verify-error-envelope.sh) | ERR-4 | T-125.4 | live |
| [`verify-handler-logging.sh`](../../scripts/website/verify-handler-logging.sh) | LOG-3 | T-125.4 | live |
| [`verify-file-length.mjs`](../../scripts/website/verify-file-length.mjs) | SIZE-1, SIZE-3 | T-125.4 | live |
| [`validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs) Enfusion DTO branch | ENF-4 | T-125.4 | live |
| **`make verify-coding-standards`** (meta target) | GO-1, GO-9, ERR-4, LOG-3, SIZE-1, SIZE-3 | T-125.4 | live |
| [`verify-editorconfig`](../../Makefile) (`editorconfig-checker` + `.editorconfig-checker.json`) | FMT-2 | T-125.5 | live |
| Prettier + `eslint-config-prettier` (`apps/website/frontend/`) | FMT-3 | T-125.5 | live |
| [`.coding-standards-allowlist.yaml`](../../.coding-standards-allowlist.yaml) | SIZE-2, SIZE-3, GO-9 structural | T-125.2/.4 | live |

## 11. Verify — replay block

`make ci-local` (T-125.1) is the single command that runs the whole gate; the ordered pieces below are
what it wraps. Each line names the rules it satisfies; `# after T-125.X` marks a piece that does not
exist until that slice ships.

```bash
make ci-local                          # whole gate (CI-1, CI-2); needs `make db-up` + `nvm use`

# 0. EditorConfig (FMT-2) — first in ci-local
make verify-editorconfig

# 1. Go format + lint (ci-local-backend)
test -z "$(gofmt -l apps/website/internal apps/website/cmd)"   # FMT-1
bash scripts/website/verify-ci1.sh                               # CI-1
cd apps/website && golangci-lint run ./...                     # GO-2,3,4,6,8 + COMP-1(Go)
cd apps/website && go build ./...
make test-it                                                   # TEST-1, GO-5, ERR-1, ERR-2, ERR-5
make verify-coding-standards                                   # GO-1, GO-9, ERR-4, LOG-3, SIZE-1, SIZE-3
# 2. Frontend (ci-local-frontend)
cd apps/website/frontend
  npm ci && npm run format:check && npm run lint && npm run build && npm test   # FMT-3, TEST-2, TS-1..7, LOG-2, COMP-1(TS), TS-5
# 3. Schema + citations (ci-local-schema)
make schema-validate                   # TEST-3, ENF-4
make verify-citations                  # TS-6, GO-7 @route route-match, ENF-3
```

> **Slice availability (2026-06-30):** **T-125 program complete** — all **38** rules in §10 are **live**
> (CI-BLOCK, CI-SCRIPT, ALLOWLIST, or MANUAL for Enfusion-only ENF-1/2). `make ci-local` mirrors
> **`ci.yml`** (backend + frontend + schema + **editorconfig** jobs).

---

## 12. Quick-reference cheat sheet

Cross-link this from [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md) (T-125.6).

| Language | Before you commit |
|----------|-------------------|
| **Go** | Handler thin (logic in `services/`, imports allowlisted)? DB errors handled or `//nolint:errcheck // best-effort:`? Errors `%w`-wrapped? Dup key → 409 via `23505`? `@route` on the handler? `golangci-lint run ./...` + `make test-it` green? |
| **TS/React** | strict-clean, no `any`? Right layer (`pages`/`features`/`ui`)? No empty catch? `@contract`/`@model` on cross-boundary types? `npm run format:check && npm run lint && npm test && npm run build` clean? |
| **Errors** | `{ error, details? }` only (no other keys)? Right status from the §4 table? Named IT per status class? |
| **Enfusion** | `enfusion-mcp` consulted? Dev toggles default off? Gates commented? Tags per DOC_STANDARDS §6–§7? Slice assigns `claude-code` to this `.c`? |
| **Always** | File ≤ 1000 L (or allowlisted) and ≤ 600 L ideally? Function complexity ≤ 15 (or inline opt-out w/ reason)? `make verify-citations` covers `@route` + `@model`; `make ci-local` is the full gate — **no commit without `make ci-local` green** (post T-125.1). Doc-comments updated in the **same commit** (DOC_STANDARDS §1)? |

---

*Defects against this standard are fixed on next edit of the affected file. Disputes resolve up the
authority ladder: running code wins, then [`CLAUDE.md`](../../CLAUDE.md), then this doc. Documentation
and tag rules live in its sibling [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md).*
