# TBD Reforger Platform

Unified monorepo for the TBD Arma Reforger milsim community: web platform, Enfusion mod framework, and shared mission schema.

## Layout

| Path | Contents |
|------|----------|
| [`apps/website/`](apps/website/) | [`api_v2/`](apps/website/api_v2/) Rust Axum + sqlx (`website-api`, compose, `.env`, seeds) · [`frontend/`](apps/website/frontend/) Leptos Trunk SPA (`website-frontend`) · [`map-engine/`](apps/website/map-engine/) and [`graphics-engine/`](apps/website/graphics-engine/) engine crates — see [`WHERE_DOES_X_GO.md`](docs/platform/WHERE_DOES_X_GO.md) |
| [`apps/mod/`](apps/mod/) | `tbd-framework` Enfusion scripts; `crf_framework/` (gitignored local reference) |
| [`contracts_v2/`](contracts_v2/) | Wire schemas, classification rules, live catalogs, golden fixtures |
| [`assets_v2/`](assets_v2/) | Terrain datasets and map glyphs, served at `/map-assets` |
| [`docs/specs/`](docs/specs/) | Design specs (Mission Creator, blueprints, UX) |
| [`docs/platform/`](docs/platform/) | Platform runbooks and architecture |
| [`docs/mod/`](docs/mod/) · [`docs/website/`](docs/website/) | App-specific docs |
| [`tools_v2/`](tools_v2/) | Tooling crates behind `cargo xtask`: `xtask` (the task runner), `ticket-engine`, `developer-tools`, `verification-core` |
| [`.ai/`](.ai/) | Ticket registry (`tickets/T-*.toml`, `wave.lock`) + pipeline artifacts |
| [`.cursor/`](.cursor/) | Cursor IDE rules + MCP (see [`docs/website/CURSOR_SETUP.md`](docs/website/CURSOR_SETUP.md)) |

## Quick start

```bash
cp apps/website/api_v2/.env.example apps/website/api_v2/.env   # if needed
cargo xtask db up
cargo xtask mk rust-api      # :8080
cargo xtask mk leptos   # :3000 (trunk serve; proxies /api + /map-assets)
```

Dev login (no Discord): `GET http://localhost:8080/api/v1/auth/dev-login?role=admin`

## Documentation

- **Agent context:** [`CLAUDE.md`](CLAUDE.md)
- **Tickets:** [ticketboard](/apps/ticketboard/README.md), the desktop viewer of the ticket registry in `.ai/tickets/`
- **Migration runbook:** [`docs/platform/MONOREPO_MIGRATION.md`](docs/platform/MONOREPO_MIGRATION.md)
