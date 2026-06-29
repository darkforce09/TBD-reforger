# T-124 — Claude Code handoff (slices .1–.3)

**Status:** **shipped @ `cd11db0`** (historical — preserved for replay)  
**Spec:** [`docs/platform/t124_dependency_upgrade.md`](../docs/platform/t124_dependency_upgrade.md)

## Shipped summary

Five commits on `main` (single-ticket mode):

| Commit | Slice | Focus |
|--------|-------|-------|
| `d81ed9c` | T-124.2 | Go modules (gin 1.12, gorm 1.31, pgx 5.10) |
| `1d85f46` | T-124.1 | Frontend npm + vitest 4 |
| `813b11d` | T-124.3 | Go 1.26, Postgres 18, CI go-version |
| `cd11db0` | T-124.3 ext | Node **26** LTS, `@types/node ^26`, drop `@tailwindcss/container-queries` |

**Final runtime:** Go 1.26 · Node 26 (`.nvmrc`) · Postgres 18 · vitest 4.1.9

## Verify replay (still valid)

```bash
nvm use   # reads .nvmrc → 26
cd apps/website/frontend && npm run build && npm run lint && npm run test   # 21/21
make build && make db-up && make test-it
make schema-codegen
cd packages/tbd-schema && npm run validate
```

## Unblocks

**T-125** — now `ready` · `./scripts/ticket brief T-125`
