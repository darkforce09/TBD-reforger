# TBD Reforger Platform

Unified monorepo for the TBD Arma Reforger milsim community: web platform, Enfusion mod framework, and shared mission schema.

## Layout

| Path | Contents |
|------|----------|
| [`website/`](website/) | Go API (`cmd/`, `internal/`), React frontend, Docker Compose, `.env` |
| [`mod/`](mod/) | `tbd-framework` Enfusion scripts, deploy/staging scripts |
| [`shared/tbd-schema/`](shared/tbd-schema/) | Mission JSON schema, golden missions, spikes |
| [`docs/specs/`](docs/specs/) | Design specs (Mission Creator, blueprints, UX) |
| [`docs/platform/`](docs/platform/) | Platform runbooks and architecture |
| [`tickets/`](tickets/) | Unified ticket registry (`registry.json`) |
| [`scripts/`](scripts/) | `./scripts/ticket` pipeline, migration helpers |

## Quick start

```bash
cp website/.env.example website/.env   # if needed
make db-up
make api      # :8080
make web      # :5173 (proxies /api)
```

Dev login (no Discord): `GET http://localhost:8080/api/v1/auth/dev-login?role=admin`

## Documentation

- **Agent context:** [`CLAUDE.md`](CLAUDE.md)
- **Ticket lead:** [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md)
- **Migration runbook:** [`docs/platform/MONOREPO_MIGRATION.md`](docs/platform/MONOREPO_MIGRATION.md)

## Original repos (archived on GitHub)

- Website: `github.com/darkforce09/TBD_Website`
- Mod: `github.com/darkforce09/tbd-reforger-platform`

Local gold copies remain on the dev machine for diff and rollback. After G5, this repo publishes to `github.com/darkforce09/TBD-Reforger`.
