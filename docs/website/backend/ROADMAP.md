# Backend — ROADMAP

Live code: [`apps/website/api_v2/`](../../../apps/website/api_v2/) (package `website-api`, Axum +
sqlx). The crate's [`README.md`](../../../apps/website/api_v2/README.md) is the atlas;
[`architecture.md`](architecture.md) keeps the archived design plan; conventions:
[`WHERE_DOES_X_GO.md`](../../platform/WHERE_DOES_X_GO.md); runbook:
[`DEV_RUNBOOK.md`](../DEV_RUNBOOK.md).

## Shipped surface, by domain

Every public URL is the literal written in the domain's `routes.rs`, with `/api/v1` in front.

| Domain (`src/<domain>/`) | Routes | Covers |
|---|---|---|
| `identity_and_access` | `/auth/*`, `/me`, `/me/link`, `/me/link/status`, `/ingest/link-confirm` | Discord OAuth2, JWT + rotating refresh, Arma link handshake, dev-login in development |
| `administration` | `/admin/users*`, `/admin/roles/sync`, `/admin/audit-logs*` | Roster, moderation, audit log with live SSE feed |
| `operations` | `/events*`, `/event-missions/*`, `/members`, `/ingest/events/{id}/roster`, `/me/deployments`, `/me/leave-requests`, `/admin/leave-requests*`, `/fire-missions*` | Calendar, ORBAT slotting, service records, mortar fire missions |
| `missions` | `/missions*`, `/registry`, `/registry/compat`, `/factions*`, `/approvals*`, `/ingest/missions`, `/admin/mission-default-overrides` | Library, versions, armory, registries, approvals, compiled export, game-server injection |
| `match_telemetry` | `/ingest/server-status`, `/ingest/match-results` | Game-server heartbeat and match reports (service token) |
| `command_center` | `/dashboard`, `/leaderboards`, `/users/{discordId}/stats` | Dashboard, leaderboard materialized view, per-player statistics |
| `community_content` | `/announcements*`, `/wiki*`, `/vehicle-database`, `/modpacks*`, `/cms/*` | Announcements with Discord push, wiki, vehicle database, modpacks, media uploads |
| `server_infrastructure` | `/servers*`, `/servers/{id}/status/stream`, `/admin/servers/{id}/rcon` | Server registry, live status SSE, RCON console through the host agent |

Cross-cutting: `core/` (configuration, database, middleware, observability at `/healthz` and
`/metrics`, the realtime hub) and `background_workers/` (token purge, rate-limit cleanup, event
lifecycle sweep, Discord role resync, leaderboard refresh, server-status publisher).

**Migrations:** [`apps/website/api_v2/migrations/`](../../../apps/website/api_v2/migrations)
(embedded, run on boot; applied files are immutable in their statements) · **Seeds:**
[`apps/website/api_v2/seeds/`](../../../apps/website/api_v2/seeds) (`cargo xtask db seed`).

## Planning

Open backend work lives in the ticket registry: [`docs/TICKET_LEAD.md`](../../TICKET_LEAD.md)
(queue) and [`docs/TICKET_REGISTRY.md`](../../TICKET_REGISTRY.md) (full registry). Contract
parity between the models, the generated contract types and the frontend DTOs is Law 9 in the
root [`CLAUDE.md`](../../../CLAUDE.md).

## Verify a change

```bash
cargo xtask db up
cargo xtask mk rust-api
curl -sf http://localhost:8080/healthz
cargo xtask db test-it
cargo xtask verify route-tags
```

API contract smoke: hit the endpoint and confirm the JSON matches the domain's `models/` serde and
`apps/website/frontend/src/v2/core/api/dto/`.
