# TBD Reforger — Coding Standards

**Status:** living
**Audience:** every engineer and AI agent that writes Go, TypeScript/React, or Enfusion code in this monorepo
**Authority:** Running code → [`CLAUDE.md`](../../CLAUDE.md) → [`docs/platform/README.md`](README.md) → **this doc** (supporting tier)
**Updated:** 2026-06-30
**Ticket:** [T-125](t125_coding_standards_enforcement.md) — authored in slice **T-125.0**; enforcement wired in **T-125.1–.5**.

> This document is the source of truth for **how code is written** across the three boundaries of
> `TBD-Reforger`. Its sibling, [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md), owns **how
> code is documented** (the cross-boundary tag vocabulary and per-language doc-comment rules). The two
> do not overlap — see the boundary matrix in §0. Like its sibling this doc is **prescriptive**: where
> it says REQUIRED, non-conforming code is a defect to fix on next edit; where it says FORBIDDEN, the
> pattern must not be introduced. It defers to running code and never overrides a rule in
> [`CLAUDE.md`](../../CLAUDE.md) or the [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md).

---

## 0. Why this exists & the documentation boundary

The repo already documents its *contracts* well (DOCUMENTATION_STANDARDS.md, ~100% Godoc, the
`@contract`/`@route` CI gates). What it lacked was a written standard for the **code itself** — when a
handler is too fat, whether a swallowed `_ = db.First(...)` is acceptable, what HTTP status a duplicate
key returns, how big a React page may grow, and which of those rules a tool actually enforces. The 2026
audit ([`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md)) surfaced the symptoms: **M6** (31 swallowed
DB/audit errors), god-files (`admin.tsx` 1628 L, `doctrine.tsx` 1288 L, `events.go` 1038 L), and
inconsistent error envelopes. This document fixes that, and §10 maps **every rule to the exact tool,
verify command, and T-125 slice** that enforces it — so "11/10" means *checked*, not *aspirational*.

### 0.1 Boundary matrix — what lives where (zero overlap)

| Concern | Owner | Notes |
|---------|-------|-------|
| `@contract` / `@route` / `@model` / `@consumer` tag grammar | **DOCUMENTATION_STANDARDS.md** §3 | Do **not** restate grammar here. |
| Godoc / TSDoc / Doxygen **doc-comment** rules (presence, prose style) | **DOCUMENTATION_STANDARDS.md** §4–§6 | Code here points up to it. |
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

Rule of thumb: **if it's about a *comment or a tag*, it's in DOCUMENTATION_STANDARDS.md; if it's about
the *code*, it's here.** When a code rule depends on a doc rule (e.g. exported-symbol Godoc), this doc
**points** to the other and does not duplicate the text.

---

## 1. The four pillars

Every rule below serves one primary pillar. The pillar is the *why*; the rule is the *what*; §10 is the
*how it's enforced*.

| Pillar | The question it answers | Example rules |
|--------|-------------------------|---------------|
| **Scalability** | Will this still be workable at 10× the size / data / team? | thin handlers (GO-1), layer boundaries (TS-2), file-size tiers (SIZE-1), MC hot-path allowlist (SIZE-2) |
| **Readability** | Can the next engineer (or agent) understand it without archaeology? | Godoc/TSDoc baseline (GO-6, TS-5), gofmt/editorconfig/Prettier (FMT-1–3), complexity cap (COMP-1) |
| **Usability** | Does the consumer (client, caller, teammate) get a correct, predictable contract? | error envelope (ERR-1), status table (ERR-2), duplicate-key classification (GO-5), surfaced FE errors (TS-4) |
| **Debuggability** | When it breaks at 02:00, can we tell *what* and *why* fast? | no swallowed DB errors (GO-2/3), `%w` wrapping (GO-4), `strict: true` (TS-1), structured logs (LOG-1), tests (TEST-1–3) |

---

## 2. Go

The backend is Gin + GORM. Handlers are the HTTP edge; `internal/services/` is the logic core;
`internal/models/` is the snake_case DB/API contract.

**REQUIRED**

- **GO-1 (Scalability) — Thin handlers; logic in `services/`.** A handler in
  [`internal/handlers/`](../../apps/website/internal/handlers) does HTTP concerns only: bind/validate
  input, check authz, call a service, map the result to a status + body. Multi-step DB work, ORBAT
  materialisation, telemetry math, and any logic reused by ≥2 handlers lives in
  [`internal/services/`](../../apps/website/internal/services). `events.go` (1038 L) is the standing
  counter-example and is tracked debt (§8).
- **GO-2 (Debuggability) — Handle DB-read errors; never silently `_ =` a query whose result you use.**
  A `_ = h.db.First(&x, …)` that then reads `x` hides "row not found" / connection errors and serves
  garbage. Check `.Error` and branch (404 / 500). Audit M6 lists the offenders (e.g.
  `deployments.go:66`).
- **GO-3 (Debuggability) — Best-effort writes are allowed *only with a rationale comment.*** Fire-and-
  forget calls where failure is genuinely non-fatal (most `services.WriteAudit(...)`) may discard the
  error, but **must** carry `//nolint:errcheck // best-effort: <why dropping is safe>` on the line. A
  bare `_ = WriteAudit(...)` with no reason is a defect — the reader cannot tell intent from accident.
- **GO-4 (Debuggability) — Wrap propagated errors with `%w`.** When returning an error up a call stack,
  add context: `fmt.Errorf("create version: %w", err)`. Never return a different, context-free error
  that erases the cause.
- **GO-5 (Usability) — Classify DB errors by code, not string match.** A unique-constraint violation is
  a **409 Conflict**, not a 500. Detect it via the Postgres SQLSTATE `23505` (`*pgconn.PgError`), not
  `strings.Contains(err.Error(), "duplicate")` (audit T6 / M6). `CreateVersion`'s semver-conflict path
  is the reference.
- **GO-6 (Readability) — Keep the Godoc baseline.** Every exported identifier carries a Godoc comment
  starting with its name. This is **owned by** [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md)
  §4 and enforced by `revive` `exported` ([`.golangci.yml`](../../apps/website/.golangci.yml)); listed
  here only so the Go rule-set is complete. Do not restate the doc grammar.

**FORBIDDEN**

- Business logic, raw SQL, or multi-table transactions written inline in a handler when a service would
  be reused (GO-1).
- `_ =` on a DB/exec call whose error is meaningful, with no `//nolint` rationale (GO-2/GO-3).
- `panic` / `log.Fatal` on a request path (verified-clean today in the audit — keep it that way).

---

## 3. TypeScript / React

Vite + React 19 + TanStack Query + Zustand. `src/types/` is the hand-written API contract mirror;
`src/api/` the axios layer; `src/hooks/` the query/mutation layer; `src/pages/` route screens;
`src/features/` self-contained domains; `src/components/ui/` shared primitives.

**REQUIRED**

- **TS-1 (Debuggability) — `"strict": true`.** [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json)
  currently runs with `strict` **off** (only `noUnusedLocals`/`noUnusedParameters`/`noFallthrough`).
  T-125.3 turns `strict: true` on and fixes the fallout. New code is written strict-clean now.
- **TS-2 (Scalability) — Respect the layer boundary.** `pages/` compose a route from hooks +
  feature/`ui` components and own *data wiring* only; reusable domain logic and heavy interactive
  surfaces live in `features/`; cross-page primitives in `components/ui/`. A page that grows its own
  business logic is refactored into a `feature` or a hook.
- **TS-3 (Debuggability) — No `any`; no unsafe non-null `!` on contract data.** Type API boundaries
  with the hand-written `types/` (or the generated `types/contract/`). `any` and `as` casts on wire
  data defeat TS-1.
- **TS-4 (Usability) — Surface API errors; never swallow a catch.** A failed mutation/query must reach
  the user with a *distinguishable* message — mirror `useMissionEditor.saveVersion` (413 → "too large",
  409 → semver, else backend `error`). A silent `catch {}` that leaves the UI in a false-success state
  is a defect (audit C3/T10/T11).
- **TS-5 (Readability) — TSDoc on contract-layer exports.** Owned by
  [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §5 and presence-gated by `eslint-plugin-jsdoc`
  ([`eslint.config.js`](../../apps/website/frontend/eslint.config.js)); listed here for completeness.

**FORBIDDEN**

- `any` (explicit or implicit-via-strict-off) on wire/contract types (TS-1/TS-3).
- Business logic in a `page` that two screens would share — extract it (TS-2).
- A `catch` that neither surfaces nor re-throws (TS-4).

---

## 4. Errors & the HTTP contract

The API speaks **one** error shape. This section is normative for every JSON handler.

- **ERR-1 (Usability) — The error envelope is `{ "error": string }`.** A validation error adds a
  `"details": string[]` array. Reference: `CreateVersion` returns
  `{ "error": "invalid mission payload", "details": [...] }`. No other top-level error keys
  (`message`, `err`, `errors`) are introduced. (List *success* bodies are `{ data, total, limit,
  offset }` per [`CLAUDE.md`](../../CLAUDE.md) §Conventions; audit logs use `next_cursor`.)
- **ERR-2 (Usability) — Use the status table.** Map conditions to status deliberately:

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

- **ERR-3 (Debuggability) — Log the failing condition on 4xx/5xx where it aids triage.** Mirror
  `CreateVersion`'s `log.Printf("CreateVersion: mission=%s status=400 … dur=%s", …)` — identifier,
  status, and duration, so a production failure is greppable. (See §9 LOG-1.)

---

## 5. Enfusion / Enforce Script — code policy

This section covers Enfusion **code** behaviour. The networked-code **tags**
(`@authority`/`@rpc`/`@replicated`/`@contract`) and doc-comment rules are **owned by**
[`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §6–§7 — do not restate them here.

- **ENF-1 (Debuggability) — Disciplined logging; dev toggles default OFF.** Use `Print(..., LogLevel.X)`
  with a level; no per-frame/per-replication-tick spam on hot paths. Any developer test switch ships
  **off** by default — `[Attribute("0")]`, not `"1"` (audit C4 `TBD_LoadoutEquipComponent`, T16
  RegistryPoc).
- **ENF-2 (Debuggability) — Annotate authority gates.** Every `if (RplSession.Mode() == RplMode.Client)
  return;` (or equivalent) carries a `// Authority only — <reason>` line so a reader knows *why* the
  branch exists. (The `//! @authority` *tag* itself is DOCUMENTATION_STANDARDS.md §7.)
- **ENF-3 (Readability) — Networked code is tagged.** Pointer only: `@authority`/`@rpc`/`@replicated`
  per [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) §7; DTOs carry `@contract` (§6.4),
  verified by the citation gate + the Enfusion DTO fixture gate (T-125.4 wires the latter into
  [`validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs)).

**Process (from [`CLAUDE.md`](../../CLAUDE.md)):** do **not** edit `apps/mod` `.c` files unless a ticket
slice explicitly assigns `claude-code` to that path, and **use `enfusion-mcp` before editing any `.c`
file** — never guess Enforce APIs. `make test-it` / the FE build do **not** cover Enfusion; mod changes
need a Workbench pass.

---

## 6. Testing — the minimum bar per layer

- **TEST-1 (Debuggability) — Go handler behaviour ⇒ integration test.** Any change to a handler's
  request/response/status behaviour requires a passing `make test-it`
  (`internal/handlers/*_integration_test.go` against a live Postgres; needs `make db-up`). `tsc` /
  `go build` proving compilation is **not** proof of contract — run the stack (see CLAUDE.md §Verifying
  changes).
- **TEST-2 (Debuggability) — FE `features/` hooks & pure utils ⇒ vitest.** Non-trivial logic in
  `features/**` hooks and pure helpers (compilers, selectors, transforms) carries a `vitest` test
  (`npm test` → `vitest run`, vitest 4.1.9). UI-only presentational components are exempt.
- **TEST-3 (Usability) — Schema/contract change ⇒ validate + fixture.** A change to a
  `packages/tbd-schema/schema/*.json` contract or a hand-written DTO is accompanied by a golden fixture
  and a green `make schema-validate` (`scripts/validate.mjs`).

---

## 7. Formatting & hygiene

- **FMT-1 (Readability) — Go is gofmt + goimports clean.** No hand-formatting; `gofmt -l apps/website`
  returns empty. CI fails on drift (T-125.1).
- **FMT-2 (Readability) — A root `.editorconfig` governs whitespace.** UTF-8, LF line endings, final
  newline, trailing-whitespace trim; indent per language (tabs for Go, 2-space for TS/JSON/YAML).
  Added in **T-125.5**.
- **FMT-3 (Readability) — Prettier is mandatory for TS/TSX/CSS.** The frontend adopts Prettier as the
  formatter-of-record (eslint keeps lint rules, drops formatting opinions). A `format` + `format:check`
  npm script is added and `prettier --check` joins lint. Configured in **T-125.5** — that slice carries
  a **one-time repo-wide reformat diff**; review it as formatting-only.

---

## 8. File size & cyclomatic complexity

- **SIZE-1 (Scalability) — Tiered size guidance (not a hard CI gate today).** A source file **> 600 lines**
  is a **review trigger** (consider a split); **> 1000 lines** must be **split or carry a
  justification** comment at the top. This is guidance enforced in review, **not** a CI failure — no
  line-count linter is wired (a future tightening may add one). Standing debt, tracked for follow-up
  split tickets:

  | File | Lines | Note |
  |------|------:|------|
  | `pages/admin.tsx` | 1628 | split by admin sub-surface (Personnel / Approvals / Audit) |
  | `pages/doctrine.tsx` | 1288 | extract the wiki split-pane helpers |
  | `handlers/events.go` | 1038 | extract ORBAT + registration into `services/` (GO-1) |

- **SIZE-2 (Scalability) — Mission Creator hot-path allowlist.** Files under
  [`src/features/tactical-map/**`](../../apps/website/frontend/src/features/tactical-map) are
  **exempt** from SIZE-1. They are deliberately dense performance code from the **T-057..T-067** scale
  program (e.g. `state/ydoc.ts` 756 L, `state/slotIconCache.ts`, `tools/useSelectTool.ts`) where the
  measured-fps contract outweighs file-length aesthetics. A change here needs a **perf note**, not a
  split; do not "clean up" a hot path without a benchmark.

- **COMP-1 (Readability) — Cyclomatic complexity ≤ 15 per function (hard gate).** A function whose
  control flow exceeds **15** independent paths is split into named helpers. Unlike the soft SIZE-1
  guideline this **fails CI**: Go via `golangci-lint` (`gocyclo` / `cyclop`, threshold 15), TypeScript
  via ESLint `complexity: ["error", 15]`. The **only** sanctioned escape is a *per-function* inline
  opt-out that carries a rationale — the same visible-exception pattern as GO-3's `//nolint:errcheck`:
  - Go: `//nolint:cyclop // <why this function must branch this much>`
  - TS: `// eslint-disable-next-line complexity -- <why>`

  This applies to **`features/tactical-map/**` as well** — the SIZE-2 file-size allowlist does **not**
  extend to complexity. A genuinely dense fps hot-path function takes the inline opt-out (so every
  exception is named and reviewable) rather than the whole directory being waved through.

---

## 9. Logging

- **LOG-1 (Debuggability) — Structured server logs on failure paths.** Follow the `CreateVersion`
  pattern: on a 4xx/5xx of consequence, log identifier + status + duration in a greppable form. Logs
  are signal, not narration — no logging inside tight loops or per-request happy-path spam.
- **LOG-2 (Debuggability) — No committed `console.log` in the frontend except dev-gated.** Debug HUDs
  and counters (e.g. `FpsCounter`, audit T12) sit behind a dev/env guard. `console.error`/`console.warn`
  for genuine error reporting is allowed. Enforced via eslint `no-console` (allow `warn`/`error`) in
  T-125.3.

---

## 10. Enforcement matrix

Every rule, the exact tool + config that checks it, the command to verify locally, and the T-125 slice
that wires it. "review" = human/agent review (no automated gate today). Slice "—" = no dedicated slice
(already enforced or review-only).

| Rule | Pillar | Statement (short) | Enforcement (tool + config) | Verify (command) | Slice |
|------|--------|-------------------|-----------------------------|------------------|-------|
| **GO-1** | Scalability | Thin handlers; logic in `services/` | review | `make test-it` | — |
| **GO-2** | Debuggability | Handle DB-read errors (no silent `_ =`) | `golangci` **errcheck** | `golangci-lint run` | T-125.2 |
| **GO-3** | Debuggability | Best-effort writes need `//nolint:errcheck // best-effort:` | `golangci` **errcheck** (`check-blank`) | `golangci-lint run` | T-125.2 |
| **GO-4** | Debuggability | Wrap propagated errors with `%w` | `golangci` **errorlint/govet** | `golangci-lint run` | T-125.2 |
| **GO-5** | Usability | Classify DB errors by `23505`, not string | review + integration test | `make test-it` | T-125.4 |
| **GO-6** | Readability | Exported Godoc baseline (→ DOC_STANDARDS §4) | `golangci` **revive** `exported` | `golangci-lint run` | — (live) / T-125.2 |
| **TS-1** | Debuggability | `"strict": true` | `tsc -b` (tsconfig.app.json) | `npm run build` | T-125.3 |
| **TS-2** | Scalability | `pages/` vs `features/` vs `ui/` boundary | review | — | — |
| **TS-3** | Debuggability | No `any` / unsafe `!` on contract data | `eslint` `no-explicit-any` + strict | `npm run lint` | T-125.3 |
| **TS-4** | Usability | Surface API errors; no silent catch | review | `npm run lint` | — |
| **TS-5** | Readability | TSDoc on contract-layer exports (→ DOC_STANDARDS §5) | `eslint-plugin-jsdoc` `require-jsdoc` | `npm run lint` | — (live) |
| **ERR-1** | Usability | `{ error, details? }` envelope | review + integration test | `make test-it` | T-125.4 |
| **ERR-2** | Usability | HTTP status table | review + integration test | `make test-it` | T-125.4 |
| **ERR-3** | Debuggability | Log condition on 4xx/5xx | review | — | T-125.4 |
| **ENF-1** | Debuggability | Log policy; dev toggles default off | Workbench review | (manual) | T-125.4 |
| **ENF-2** | Debuggability | `// Authority only — <reason>` on gates | review | (manual) | — |
| **ENF-3** | Readability | Networked-code tags (→ DOC_STANDARDS §6–§7) | citation gate + DTO fixture | `make verify-citations` | T-125.4 |
| **TEST-1** | Debuggability | Go handler change ⇒ `make test-it` | `ci.yml` backend job | `make test-it` | T-125.1 |
| **TEST-2** | Debuggability | FE features hooks/utils ⇒ vitest | `ci.yml` frontend job | `npm test` | T-125.1 |
| **TEST-3** | Usability | Schema change ⇒ validate + fixture | `ci.yml` schema job | `make schema-validate` | T-125.1 |
| **FMT-1** | Readability | gofmt + goimports clean | `ci.yml` `gofmt -l` | `gofmt -l apps/website` | T-125.1 |
| **FMT-2** | Readability | Root `.editorconfig` | `.editorconfig` (+ optional checker) | file present | T-125.5 |
| **FMT-3** | Readability | Prettier for TS/TSX/CSS | `prettier --check` | `npm run format:check` | T-125.5 |
| **SIZE-1** | Scalability | >600 L review / >1000 L split-or-justify | review (no CI gate) | (manual) | — |
| **SIZE-2** | Scalability | `tactical-map/**` hot-path allowlist | review (perf note) | (manual) | — |
| **COMP-1** | Readability | Cyclomatic complexity ≤ 15/function (hard gate) | `golangci` **gocyclo/cyclop** (15) · `eslint` `complexity:["error",15]` | `golangci-lint run` · `npm run lint` | T-125.2 (Go) / T-125.3 (TS) |
| **LOG-1** | Debuggability | Structured server logs on failure | review | — | — |
| **LOG-2** | Debuggability | No committed FE `console.log` (dev-gated only) | `eslint` `no-console` | `npm run lint` | T-125.3 |

**Count by pillar:** Scalability 4 · Readability 7 · Usability 4 · Debuggability 13 · **28 total.**

---

## 11. Verify — replay block

The full local mirror of CI. `make ci-local` (added in T-125.1) wraps the lot; the individual commands
are the pieces it runs.

```bash
make ci-local                       # (T-125.1) one command = the whole gate below

# — or run the pieces —
make db-up                          # Postgres for integration tests
make build                          # go build ./... + frontend tsc + vite build
make test-it                        # Go handler integration tests (TEST-1, ERR-*, GO-5)
cd apps/website/frontend
  npm run lint                      # eslint: TS-3/TS-5/LOG-2/COMP-1 + TSDoc gate
  npm test                          # vitest run (TEST-2)
  npm run build                     # tsc -b strict (TS-1) + vite
  npm run format:check              # prettier --check (FMT-3)   [after T-125.5]
make schema-validate                # golden fixtures (TEST-3)
make verify-citations               # @contract links resolve (ENF-3, DOC_STANDARDS §10)
cd apps/website && golangci-lint run # errcheck/govet/staticcheck/revive + gocyclo/cyclop (GO-2..GO-6, COMP-1) [after T-125.2]
gofmt -l apps/website               # empty = FMT-1 clean
```

> **Note (slice availability):** `make ci-local`, the hardened `golangci` set, `strict: true`, and the
> Prettier script do not exist yet at T-125.0 — they land in T-125.1–.5. The block above is the *target*
> replay; until each slice ships, run the pieces that already exist.

---

## 12. Quick-reference cheat sheet

Cross-link this from [`AGENT_COMMIT_CHECKLIST.md`](../website/AGENT_COMMIT_CHECKLIST.md) (Cursor, T-125.6).

| Language | Before you commit |
|----------|-------------------|
| **Go** | Handler thin (logic in `services/`)? DB errors handled or `//nolint:errcheck // best-effort:`? Errors `%w`-wrapped? Dup key → 409 via `23505`? `gofmt`/`golangci` clean? `make test-it` green? |
| **TS/React** | strict-clean, no `any`? Logic in the right layer (`pages`/`features`/`ui`)? API errors surfaced? `npm run lint && npm test && npm run build` clean? |
| **Errors** | `{ error, details? }` only? Right status from the §4 table? |
| **Enfusion** | `enfusion-mcp` consulted? Dev toggles default off? Gates commented? Tags per DOC_STANDARDS §6–§7? Slice assigns `claude-code` to this `.c`? |
| **Always** | File < 600 L (or justified / `tactical-map/**` allowlisted)? Function complexity ≤ 15 (or inline opt-out w/ reason)? Doc-comments updated in the **same commit** (DOC_STANDARDS §1)? |

---

*Defects against this standard are fixed on next edit of the affected file. Disputes resolve up the
authority ladder: running code wins, then [`CLAUDE.md`](../../CLAUDE.md), then this doc. Documentation
and tag rules live in its sibling [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md).*
