# T-125 — Coding standards + 11/10 enforcement

**Ticket:** T-125 · **Program:** platform · **Status:** **ready** (T-124 shipped @ `cd11db0`)  
**Depends on:** T-124 (met) · **Active slice:** T-125.0 · **Handoff:** [`.ai/artifacts/t125_claude_code_handoff.md`](../../.ai/artifacts/t125_claude_code_handoff.md)

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
| **backend** | Postgres 18 service (match compose: tbd/tbd, port 5434); Go **1.26**; `go build`, `make test-it` |
| **frontend** | Node **26**; `npm ci`, `npm run lint`, `npm run build`, `npm test` |
| **schema** | `npm run validate`, `make verify-citations` |

Add **`make ci-local`** (or `make check`) mirroring CI.

**Verify:** workflow green locally where possible; push to main.

---

## T-125.2 — golangci full gate

Harden [`apps/website/.golangci.yml`](../../apps/website/.golangci.yml):

- Add **errcheck**, **govet**, **staticcheck** (in addition to revive `exported`)
- **Remove `only-new-issues: true`** from [`contracts.yml`](../../.github/workflows/contracts.yml) (or merge golangci into `ci.yml` and dedupe)
- Fix **all** linter findings repo-wide

**Verify:** `golangci-lint run ./...` clean; `make test-it`.

---

## T-125.3 — TypeScript strict + eslint tags

- Enable **`strict: true`** in [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json); fix all errors (expect MC + pages touch)
- Harden [`eslint.config.js`](../../apps/website/frontend/eslint.config.js): enforce **`@contract` / `@model`** on cross-boundary exports (custom rule or extend [`verify-contract-citations.mjs`](../../packages/tbd-schema/scripts/verify-contract-citations.mjs))

**Verify:** `npm run build && npm run lint && npm test`.

---

## T-125.4 — Routes, errors, DTO gate

- Complete **`@route`** on all exported handlers in [`internal/handlers/`](../../apps/website/internal/handlers/)
- Expand **`@model` / `@contract`** on [`types/api/index.ts`](../../apps/website/frontend/src/types/api/index.ts) where types mirror GORM models
- Fix high-impact **`_ = db.First` / `_ = WriteAudit`** per CODING_STANDARDS error policy ([`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md) M6)
- Wire **§10 Enfusion DTO fixture gate** in [`validate.mjs`](../../packages/tbd-schema/scripts/validate.mjs) (promised in DOCUMENTATION_STANDARDS, not yet implemented)

**Verify:** `make test-it`; citation + validate scripts exit 0.

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
- [ ] **TypeScript `strict: true`** — build clean
- [ ] **Every handler** has `@route` in Godoc; cross-boundary TS types have `@model`/`@contract` where applicable
- [ ] Citation verifier + any new tag verifiers exit 0
- [ ] Replay commands documented in spec and DEV_RUNBOOK

---

## Risk notes

- **Full gate on day one** produces a **large diff** (especially `strict: true` + errcheck + removing `only-new-issues`). Budget one heavy PR.
- **Postgres 18** in `ci.yml` must match T-124 compose image.
