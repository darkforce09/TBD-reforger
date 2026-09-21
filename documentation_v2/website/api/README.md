# Backend REST & Realtime API (`website/api/`)

The backend API service is an asynchronous Rust service powered by Axum, SQLx, and PostgreSQL, serving HTTP endpoints on port 8080 and broadcasting real-time state changes via Server-Sent Events (SSE). The crate is organised by domain, not by layer: `core/` holds what every domain depends on, `background_workers/` the interval tasks the binary arms at boot, and each domain owns its route table, handlers, services and models.

## Subsystems
- `core/`: configuration, database pool and migrations, error mapping, JWT and token primitives, the router that merges the domain route tables under `/api/v1`, middleware (authentication tiers, CORS, request ids, rate limiting), observability (`/healthz`, `/metrics`), the SSE realtime hub, and the shared wire formats.
- `background_workers/`: refresh-token purge, rate-limit bucket cleanup, event lifecycle sweep, Discord role resync, leaderboard refresh, server-status publisher.
- `identity_and_access/`: Discord OAuth2, dev-login for local development, session issuance and rotating refresh, the profile, the Arma link handshake.
- `administration/`: member roster, moderation actions, the audit log and its live feed.
- `operations/`: event calendar, ORBAT slotting and registration, service records, leave requests, fire missions.
- `missions/`: scenario library, versions, armory, registries, approvals, the compiled export, game-server injection; `contract/` holds the schema validators and the generated contract types.
- `match_telemetry/`: game-server heartbeat and match-result ingest.
- `command_center/`: dashboard, leaderboards, per-player statistics.
- `community_content/`: announcements with Discord push, wiki, vehicle database, modpacks, media uploads.
- `server_infrastructure/`: dedicated-server registry, live status stream, RCON console through the host agent.
- `tests/architecture_rules.rs` and `tests/prose_rules.rs`: the layout and prose rules, checked against the source text on every unit-test run.

## Code Mapping
- Source: `apps/website/api_v2/`
- Configuration: `apps/website/api_v2/.env`
- Migrations: `apps/website/api_v2/migrations/`
- Atlas: `apps/website/api_v2/README.md`
