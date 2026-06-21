# Backend documentation — master index

**Status:** living  
**Audience:** backend developers, API agents  
**Authority:** [`internal/models/`](../../internal/models/) (GORM JSON tags) → handlers → this tree  
**Updated:** 2026-06-20

Single front door for backend documentation and code entrypoints.

## Living

| Link | Purpose |
|------|---------|
| [`docs/backend/architecture.md`](architecture.md) | Target schema + design (verify vs live models post T-008) |
| [`DEV_RUNBOOK.md`](../../DEV_RUNBOOK.md) | db-up, api, web, dev-login, test-it, seeds |
| [`internal/models/`](../../internal/models/) | **API contract** — GORM struct JSON tags |
| [`internal/handlers/`](../../internal/handlers/) | HTTP handlers (one file per resource) |
| [`internal/db/migrations/`](../../internal/db/migrations/) | SQL migrations (pre-AutoMigrate) |
| [`internal/db/seeds/`](../../internal/db/seeds/) | Seed SQL (Discord role mappings) |
| [`cmd/api/`](../../cmd/api/) | API entrypoint |
| [`cmd/seed/`](../../cmd/seed/) | Seed runner |
| [`.env.example`](../../.env.example) | Environment variable reference |
| [`docs/platform/registration_flow.md`](../platform/registration_flow.md) | ORBAT registration design (implemented T-008–T-010) |

## Archive / deferred

| Link | Note |
|------|------|
| [`docs/backend/architecture.md`](architecture.md) | May pre-date T-008 campaign refactor — verify against `internal/models/` |
| *(future)* `docs/backend/api.md` | Per-handler reference — not built yet |

## Related

- [Platform doc hub](../README.md)
- [Frontend master](../frontend/README.md)
- [Archive master](../archive/README.md)
