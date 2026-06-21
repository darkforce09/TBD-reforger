# TBD Reforger Platform

Web suite for the TBD Arma Reforger milsim community: Discord auth, event/ORBAT scheduling, mission library, server telemetry, doctrine wiki, CMS, and admin tooling.

## Quick start

```bash
make db-up    # Postgres on :5434
make api      # Go API on :8080
make web      # Vite dev server on :5173
```

Full commands: [`DEV_RUNBOOK.md`](DEV_RUNBOOK.md)

## Documentation

**All docs:** [`docs/README.md`](docs/README.md)

| Role | Start here |
|------|------------|
| Frontend | [`docs/frontend/README.md`](docs/frontend/README.md) |
| Backend | [`docs/backend/README.md`](docs/backend/README.md) |
| AI agents | [`CLAUDE.md`](CLAUDE.md) |
| Archive / mockups | [`docs/archive/README.md`](docs/archive/README.md) |

## Stack

- **Backend:** Go (Gin + GORM), PostgreSQL — `internal/`, `cmd/api/`
- **Frontend:** React 19 + TypeScript + Vite — `frontend/`
