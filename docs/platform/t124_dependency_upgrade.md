# T-124 — Dependency & toolchain upgrade (latest everything)

**Ticket:** T-124 · **Program:** platform · **Status:** ready  
**Executor:** Claude Code · **Execution:** **3 commits on `main`** (single-ticket mode — no feature branch)  
**Handoff:** [`.ai/artifacts/t124_claude_code_handoff.md`](../../.ai/artifacts/t124_claude_code_handoff.md)

## In one sentence

Bring the monorepo to the newest supported versions: frontend npm (including **vitest 4**), Go modules, **Go 1.26**, **Node 24**, and **Postgres 18** dev image — with staged commits and full verify replay.

## Context

Live recon shows the repo is already on the newest **major** of nearly everything (React 19, Vite 8, Tailwind 4, ESLint 10, TS 6, deck.gl 9, gin 1.10, gorm 1.25). T-124 is mostly in-caret minor/patch bumps plus a handful of deliberate jumps — not a wall of breaking majors.

### Locked decisions

| Decision | Choice |
|----------|--------|
| Breaking majors | All latest **incl. vitest 3→4** |
| `@types/node` | Stay **^24** (match Node 24 runtime — do not chase @types/node 26) |
| Toolchain | **Go 1.25→1.26**, **Node 20→24** |
| Postgres dev | **16→18** (`postgres:18-alpine`) |

### Toolchain reality

- Local Go is already **1.26.4** (ahead of `go.mod` 1.25)
- Local Node was **20.20.2** (EOL; CI already on 24)
- CI: checkout@v7, setup-node@v6/24, setup-go@v6/1.25, golangci-lint-action@v9

---

## Prerequisite (human, before slice T-124.1)

Install **Node 24** locally (`nvm install 24 && nvm use 24` or system pkg). Commit 3 adds **`.nvmrc` = 24** at repo root.

---

## Slice plan (maps to commits)

| Slice | Commit | Scope |
|-------|--------|-------|
| **T-124.1** | 1 | Frontend deps (`apps/website/frontend/`) |
| **T-124.2** | 2 | Backend Go deps (`apps/website/`) |
| **T-124.3** | 3 | Toolchain + infra + CLAUDE version lines |

Advance after each slice verifies: `./scripts/ticket advance-slice T-124`

**Do not** edit `.ai/tickets/registry.json` or narrative docs in Claude commits — Cursor registers before start and marks shipped after verify.

---

## T-124.1 — Frontend deps (Commit 1)

**Dir:** `apps/website/frontend/` (requires Node 24)

1. `npm update` — in-caret bumps: @base-ui/react 1.6, @deck.gl/* + deck.gl + @luma.gl 9.3.5 lockstep, tailwindcss/@tailwindcss/vite 4.3.2, @tanstack/react-query 5.101.2, @tanstack/react-virtual 3.14.4, @vitejs/plugin-react 6.0.3, axios 1.18.1, eslint 10.6.0, globals 17.7.0, lucide-react 1.22.0, react-hook-form 7.80.0, react-router-dom 7.18.1, shadcn 4.12.0, typescript-eslint 8.62.1, vite 8.1.0.
2. **vitest 3→4** (breaking): `package.json` `vitest ^3.2.0` → `^4.x`, reinstall; reconcile [`vitest.config.ts`](../../apps/website/frontend/vitest.config.ts) (environment: node, map-assets alias); re-run **21** tactical-map tests.
3. Do **not** bump `@types/node` past ^24.
4. TypeScript ~6.0.2 already latest — leave tilde + `ignoreDeprecations: "6.0"` in [`tsconfig.app.json`](../../apps/website/frontend/tsconfig.app.json).
5. **Optional cleanup:** drop `@tailwindcss/container-queries` if unused under Tailwind v4 native container queries; check `@types/supercluster` vs supercluster 8.
6. **Optional:** `npm update` in [`packages/tbd-schema`](../../packages/tbd-schema) if outdated; `npm run validate`.
7. Commit `package.json` + `package-lock.json` (+ tbd-schema lockfiles if touched).

**Verify:**

```bash
cd apps/website/frontend && npm run build && npm run lint && npm run test
# expect 21/21
cd packages/tbd-schema && npm run validate  # if touched
```

---

## T-124.2 — Backend Go deps (Commit 2)

**Dir:** `apps/website/` (Go 1.26.4)

1. `go get -u ./... && go mod tidy` — gin 1.10→1.12, gorm 1.25→1.31, gorm.io/driver/postgres 1.5→1.6, gorm.io/datatypes 1.2.7, jackc/pgx/v5 5.5→5.10, golang.org/x/*; jwt/v5, uuid, jsonschema/v6, bluemonday already latest.
2. **Watch gorm 1.31** — highest behavior risk; T-123 `CreateVersion` + [`internal/contract/validate.go`](../../apps/website/internal/contract/validate.go) must pass.
3. Commit `go.mod` + `go.sum`.

**Verify:**

```bash
make build
make db-up && make test-it
```

---

## T-124.3 — Toolchain + infra (Commit 3)

1. [`go.mod`](../../apps/website/go.mod): `go 1.25.0` → `1.26.0` (no explicit toolchain line).
2. [`.github/workflows/contracts.yml`](../../.github/workflows/contracts.yml): `go-version: "1.25"` → `"1.26"` (codegen-drift + go-doc-lint jobs). Leave actions at v7/v6/v9. [`schema.yml`](../../.github/workflows/schema.yml) unchanged unless runner moves.
3. Add **`.nvmrc`** = `24` at repo root.
4. [`docker-compose.yml`](../../apps/website/docker-compose.yml): `postgres:16-alpine` → `18-alpine`.
5. **Version doc sync only:** [`CLAUDE.md`](../../CLAUDE.md) Go 1.25→1.26, Node 20→24; optionally [`scripts/deploy/deploy.env.example`](../../scripts/deploy/deploy.env.example) node 18+→24+. Full DEV_RUNBOOK prose → Cursor after ship.

**Verify:**

```bash
make schema-codegen   # git status clean
make db-down          # remove compose volume
make db-up && make seed && make api  # PG18 migrations + boot
```

**End-to-end smoke:** `make api` + `make web`, dev-login, load page + mission editor (deck.gl/yjs).

---

## Critical files

- `apps/website/frontend/package.json`, `package-lock.json`, `vitest.config.ts`
- `apps/website/go.mod`, `go.sum`
- `.github/workflows/contracts.yml`
- `apps/website/docker-compose.yml`
- `.nvmrc` (new), `CLAUDE.md`

---

## Risk & rollback

| Risk | Mitigation |
|------|------------|
| vitest 4 | Isolated to commit 1; fix config + 21 tests |
| gorm 1.31 | Isolated to commit 2; `make test-it` |
| Postgres 18 | Volume re-init; dev data reseedable via `make seed` |
| Rollback | Restore lockfiles/manifests per commit |

Staged commits isolate blast radius.

---

## Out of scope

- Arma Reforger / Tools binary versions (Steam)
- **Mod / enfusion-mcp pin** — deferred (optional follow-up; not blocking T-125)
- Coding-standard refactors → **T-125**
- Registry / narrative doc sync → **Cursor** after ship

---

## Unblocks

**T-125** — coding standards + full CI enforcement (depends on T-124)
