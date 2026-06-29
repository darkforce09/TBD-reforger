# T-124 — Dependency & toolchain upgrade (latest everything)

**Ticket:** T-124 · **Program:** platform · **Status:** **shipped @ `cd11db0`**
**Handoff:** [`.ai/artifacts/t124_claude_code_handoff.md`](../../.ai/artifacts/t124_claude_code_handoff.md) (historical)

## In one sentence

Upgraded the monorepo to the newest supported versions: frontend npm (including **vitest 4**), Go modules (gin/gorm/pgx), **Go 1.26**, **Node 26**, and **Postgres 18** dev image — five commits on `main`, full verify replay green.

## Shipped commits

| Commit | Slice | Summary |
|--------|-------|---------|
| `d81ed9c` | T-124.2 | Go modules — gin 1.12, gorm 1.31.2, pgx 5.10, postgres driver 1.6 |
| `1d85f46` | T-124.1 | Frontend npm — vitest 4.1.9, deck.gl 9.3.5 lockstep, vite 8.x |
| `813b11d` | T-124.3 | Toolchain — Go 1.26, `.nvmrc` 24→26 path, Postgres 18, CI go-version 1.26 |
| `cbe3664` | — | Registry ticket sync (premature slice status — corrected in Cursor doc pass) |
| `cd11db0` | T-124.3 ext | **Node 26** (latest LTS): CI node-version 26, `@types/node ^26`, dropped unused `@tailwindcss/container-queries` |

**Note:** Original spec targeted Node 24; ship commit `cd11db0` moved runtime to **Node 26** to match latest LTS. `@types/node` tracks runtime (^26).

## Final toolchain (authoritative)

| Surface | Version |
|---------|---------|
| **Go** | 1.26 (`apps/website/go.mod`) |
| **Node** | 26 (`.nvmrc`, CI `setup-node`, `@types/node ^26`) |
| **Postgres dev** | 18-alpine (`apps/website/docker-compose.yml`) |
| **vitest** | 4.1.9 |
| **gin / gorm / pgx** | 1.12 / 1.31.2 / 5.10 |

CI: [`.github/workflows/contracts.yml`](../../.github/workflows/contracts.yml) + [`schema.yml`](../../.github/workflows/schema.yml) — Node **26**, Go **1.26**.

## Verification (replay @ ship)

```bash
# Node 26 (nvm use per .nvmrc)
cd apps/website/frontend && npm run build && npm run lint && npm run test   # 21/21
make build
make db-up && make test-it
make schema-codegen   # no drift
cd packages/tbd-schema && npm run validate
```

**Postgres 18:** after first upgrade, re-init local volume if migrations fail on stale PG16 data:

```bash
make db-down
# remove compose volume (podman/docker volume rm …)
make db-up && make seed
```

## Slice plan (historical)

| Slice | Commit | Status |
|-------|--------|--------|
| **T-124.1** | `1d85f46` | shipped |
| **T-124.2** | `d81ed9c` | shipped |
| **T-124.3** | `813b11d` + `cd11db0` | shipped |

## Out of scope (deferred)

- Arma Reforger / Tools binary versions (Steam)
- **Mod / enfusion-mcp pin** — optional follow-up
- Coding-standard refactors → **T-125**

## Unblocks

**T-125** — coding standards + full CI enforcement (`ready` after this ship)
