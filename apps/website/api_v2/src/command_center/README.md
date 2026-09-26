# Command center domain

The [API](/documentation_v2/glossary/a_to_f.md#api)'s
[command center](/documentation_v2/glossary/a_to_f.md#command-center) domain: the platform's read surfaces.
It serves the members' dashboard, which composes many best-effort lookups into one answer, the
ranked community leaderboards and one player's statistics card, and it owns the derived figures
behind them. The ingest that produces those figures belongs to `match_telemetry`, and the
[events](/documentation_v2/glossary/a_to_f.md#event) they are attributed to belong to `operations`.

## Contents

```text
apps/website/api_v2/src/command_center/
├── handlers/  the dashboard, leaderboard and statistics card reads
├── mod.rs     the module tree; re-exports `routes`
├── routes.rs  the domain's `/api/v1` route table
└── services/  the member statistics recomputation and the leaderboard view refresh
```

## How it works

Every route takes `AuthUser`. The domain owns no models: the handlers project rows through the
services and models of the domains they read. The leaderboards and the statistics card read the
`leaderboard_totals` materialized view; `services/leaderboard_view.rs` refreshes it on a schedule
through the leaderboard worker, and inside the match-results and identity-link transactions.
`services/user_stats.rs` recomputes each member's `total_deployments` and `attendance_rate` inside
those transactions and the event administration ones, so the figures commit with the facts they
summarise.

## Public surface

- `routes::routes()`: the table `core::http_router` merges under `/api/v1`, one route each, all
  `AuthUser`:
  - `GET /api/v1/dashboard`: the next event, the caller's assignment, server status, modpack, news.
  - `GET /api/v1/leaderboards`: the ranked board for one category, searchable by name.
  - `GET /api/v1/users/{discordId}/stats`: one player's statistics card.
- `services::leaderboard_view`: `refresh_leaderboard`, run by the leaderboard worker, and
  `refresh_leaderboard_on_connection`, called by the match-results ingest and identity linking.
- `services::user_stats`: `recompute_user_stats_on_connection`, called by the match-results
  ingest, identity linking and event administration, and `ATTENDANCE_RATE_SQL`, which the account
  lookup in `identity_and_access` reads.

## Boundaries

- Depends on: `core` (the application state, errors, the `AuthUser` extractor, wire formats);
  `identity_and_access`, `missions`, `community_content`, `operations` and `server_infrastructure`
  for the rows the dashboard and the card read; `administration` for the warning a failed
  best-effort recomputation records.
- Used by:
  - `core::http_router`, which merges the route table, and the `leaderboard_refresher` worker in
    `apps/website/api_v2/src/background_workers/`;
  - `match_telemetry`, `identity_and_access` and `operations`, through the services above;
  - over HTTP, the dashboard and the leaderboard pages in `apps/website/frontend/src/v2/pages/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/website/api_v2/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); the view refresh and the counter
  recomputation each have one home, in `services/`.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md)
  — `LEADERBOARD_REFRESH_INTERVAL_SECS`,
  the cadence of the scheduled leaderboard refresh.
- [Reservation and attendance separation](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md)
  — what counts as attendance, which the statistics summarise.
