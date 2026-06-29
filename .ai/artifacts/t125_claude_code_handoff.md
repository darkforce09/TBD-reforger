# T-125 — Claude Code handoff (slices .0–.5)

**Status:** **ready** (T-124 shipped @ `cd11db0`)  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md)  
**Authority:** new **`CODING_STANDARDS.md`** (Claude authors in T-125.0) + existing [`DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md)

## Prerequisite (met)

**T-124 shipped** — deps and toolchain on latest: Go **1.26**, Node **26**, Postgres **18**, vitest **4.1.9**, gin **1.12**, gorm **1.31.2**.

## Slice order

| # | Slice | Focus |
|---|-------|-------|
| 0 | **T-125.0** | Write `docs/platform/CODING_STANDARDS.md` |
| 1 | **T-125.1** | `ci.yml` + Postgres 18 service + `make ci-local` |
| 2 | **T-125.2** | golangci errcheck/govet/staticcheck; drop `only-new-issues`; fix all Go lint |
| 3 | **T-125.3** | TS `strict: true` + eslint `@contract`/`@model` enforcement + fixes |
| 4 | **T-125.4** | Complete `@route` on handlers; error-handling; Enfusion DTO fixture gate |
| 5 | **T-125.5** | `.editorconfig` / Prettier (if in standard) |
| 6 | **T-125.6** | **cursor-docs** — registry shipped, hub, CLAUDE §Done, ticket sync |

Advance: `./scripts/ticket advance-slice T-125` after each verify.

## Verify (full replay after T-125.5)

```bash
nvm use   # Node 26
make ci-local   # once added in T-125.1
make test-it
cd apps/website/frontend && npm run build && npm run lint && npm run test
cd packages/tbd-schema && npm run validate
make verify-citations
golangci-lint run ./...
```

## Return to Cursor

T-125.6 is **cursor-docs** — Claude stops after .5; Cursor syncs registry + narrative docs.
