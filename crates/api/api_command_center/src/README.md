# Command center domain

The [API](/documentation/glossary/a_to_f.md#api)'s
[command center](/documentation/glossary/a_to_f.md#command-center) domain: the platform's read surfaces.
It serves the members' dashboard, which composes many best-effort lookups into one answer, the
ranked community leaderboards and one player's statistics card, and it owns the derived figures
behind them. The ingest that produces those figures belongs to `api_match_telemetry`, the
recomputation of the figures to `api_member_activity`, and the
[events](/documentation/glossary/a_to_f.md#event) they are attributed to belong to `api_operations`.

## Contents

```text
crates/api/api_command_center/src/
├── error.rs    the crate's `Error` and `Result`, converting into `ApiError`
├── handlers/   the dashboard, leaderboard and statistics card reads
├── lib.rs      the crate root: the module tree; re-exports `routes`, `Error` and `Result`
├── prelude.rs  the leaderboard and fleet overview names a caller imports
├── routes.rs   the domain's `/api/v1` route table
└── services/   the fleet overview the dashboard reads
```

## How it works

Every route takes `AuthUser`. The domain owns no models: the handlers project rows through the
services and models of the domains they read. The leaderboards and the statistics card read the
`leaderboard_totals` materialized view and the members' `total_deployments` and
`attendance_rate`, which the member activity crate (`crates/api/api_member_activity/src/`)
keeps current inside the match-results, identity-link and event administration transactions.

The dashboard's `fleet` block comes from `services/fleet_overview.rs`: the configured fleet is the
set of servers with `is_active = true`, listed by name then id with each one's status (a server
with no status row has none and counts as offline), and totals that count online servers, their
players and capacity, and sum every reported telemetry queue's backlog and drops.

## Public surface

- `routes()`: the table the API's router (`crates/api/api_server/src/router.rs`) merges under
  `/api/v1`, one route each, all
  `AuthUser`:
  - `GET /api/v1/dashboard`: the next event, the caller's assignment, the configured fleet with
    its totals, modpack, news.
  - `GET /api/v1/leaderboards`: the ranked board for one category, searchable by name.
  - `GET /api/v1/users/{discordId}/stats`: one player's statistics card.
- `handlers::leaderboards`: `get_leaderboards`, `LeaderboardQuery` and `LeaderboardRow`, which
  the integration suites call.
- `services::fleet_overview`: `load_fleet_overview` and its `FleetOverview`, read by the dashboard
  handler.
- `Error` and `Result` (a failed fleet read, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state` (the application state), `api_foundation` (the handler error, the path
  parameters), `api_http_layer` (the `AuthUser` extractor), `api_identifiers` and
  `fleet_wire_contract` (the RFC 3339 spelling); `api_identity_and_access`, `api_missions`,
  `api_community_content`, `api_operations` and `api_server_infrastructure` for the rows the
  dashboard and the card read.
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route table;
  - the API's integration suites in `crates/api/api_server/tests/`;
  - over HTTP, the dashboard and the leaderboard pages in the page crates under `crates/frontend/pages/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges; every handler
  carries its `/// @route` tag; the view refresh and the counter
  recomputation each have one home, in `services/`.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [API environment variables](/documentation/crates/api/api_server/environment_variables.md)
  — `LEADERBOARD_REFRESH_INTERVAL_SECS`,
  the cadence of the scheduled leaderboard refresh.
- [Match telemetry, fleet status and derived statistics](/documentation/crates/api/api_server/design_notes/telemetry.md)
  — the fleet block and when the derived statistics are recomputed.
- [Reservation and attendance separation](/documentation/crates/api/api_server/design_notes/reservation_attendance.md)
  — what counts as attendance, which the statistics summarise.
