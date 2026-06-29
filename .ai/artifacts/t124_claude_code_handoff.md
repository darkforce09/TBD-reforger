# T-124 — Claude Code handoff (slices .1–.3)

**Status:** ready · **Spec:** [`docs/platform/t124_dependency_upgrade.md`](../docs/platform/t124_dependency_upgrade.md)

## Execution model

**Single-ticket mode — 3 commits directly on `main`**, tag **T-124**, `Co-Authored-By` trailer each. **No feature branch.**

**Prerequisite (human):** Node **24** locally (`nvm install 24 && nvm use 24`) before slice T-124.1.

**Do not** edit `.ai/tickets/registry.json` or narrative docs — Cursor registers before start and marks shipped after verify. Commit 3 may update **version numbers only** in `CLAUDE.md`.

## Slice order

| # | Slice | Commit | Focus |
|---|-------|--------|-------|
| 1 | **T-124.1** | 1 | Frontend npm + vitest 4 |
| 2 | **T-124.2** | 2 | Go modules (`go get -u ./...`) |
| 3 | **T-124.3** | 3 | Go 1.26, Node .nvmrc, PG18, CI go-version, CLAUDE version lines |

Advance: `./scripts/ticket advance-slice T-124` after each verify.

## Locked rules

- **`@types/node`:** stay **^24** (do not bump to 26)
- **vitest:** explicit 3→4 bump (breaking)
- **gorm 1.31:** highest behavior risk — `make test-it` must pass
- **Postgres 18:** volume re-init after compose bump

## Verify (replay per slice)

**T-124.1:**

```bash
cd apps/website/frontend && npm run build && npm run lint && npm run test
# 21/21 tactical-map tests
```

**T-124.2:**

```bash
make build
make db-up && make test-it
```

**T-124.3:**

```bash
make schema-codegen   # git status clean
make db-down          # remove compose volume
make db-up && make seed && make api
make api + make web   # dev-login + mission editor smoke
```

## Out of scope

- Mod / enfusion-mcp pin (deferred)
- Coding-standard refactors → T-125

## Return to Cursor

Per-slice verify output → Cursor marks shipped + DEV_RUNBOOK Postgres 18 note + `./scripts/ticket sync`.
