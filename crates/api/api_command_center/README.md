# API command center

The `api_command_center` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[command center](/documentation/glossary/a_to_f.md#command-center) domain. It holds the platform's
read surfaces for members: the dashboard, which composes the next
[event](/documentation/glossary/a_to_f.md#event), the caller's assignment, the configured
game-server fleet, the current modpack and the latest announcements into one answer; the ranked
community leaderboards; and one player's statistics card, with the `/api/v1` route table the API's
router merges.

## Contents

```text
crates/api/api_command_center/
├── Cargo.toml  the package: `api_state`, `api_http_layer`, the five domain crates it reads, `fleet_wire_contract`, sqlx (`postgres`), axum, layout tier 10
└── src/        the route table, the handlers, the fleet overview service, the error and the prelude
```

## How it works

Every route takes `AuthUser`. The crate owns no table and no model: the leaderboards and the
statistics card read the `leaderboard_totals` materialized view and the members' derived figures,
which `api_member_activity` keeps current, and the dashboard reads the rows of the domains that
own them. Each dashboard lookup is best-effort, so a missing piece is `null` rather than a failed
dashboard. A leaderboard category maps to its ordering only through a fixed list, and every
ordering ends on the Discord id, so tied scores page deterministically. The source tree README has
the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_command_center
cargo clippy -p api_command_center --all-targets -- -D warnings
cargo xtask db test-it --test user_stats_service --test leaderboards_paging --test dashboard_reads --test route_acceptance_administration_center_content
```

The unit tests cover the leaderboard categories and their orderings and the fleet totals; the
reads and the routes are proved against Postgres by the API's integration suites.

## Configuration

No feature and no variable of its own. The cadence of the leaderboard refresh it reads is the
API's `LEADERBOARD_REFRESH_INTERVAL_SECS`
([API environment variables](/documentation/crates/api/api_server/environment_variables.md)).

## Public surface

- `routes()`: the domain's `/api/v1` route table (`/dashboard`, `/leaderboards`,
  `/users/{discordId}/stats`).
- `handlers`: the dashboard, the leaderboards (`get_leaderboards`, `LeaderboardQuery`,
  `LeaderboardRow`) and the statistics card, each with its `/// @route` tag.
- `services::fleet_overview`: `load_fleet_overview`, `FleetOverview`, `FleetServer`,
  `FleetTotals` and `fleet_totals`.
- `Error` and `Result` (a failed fleet read, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_http_layer`, `api_foundation`, `api_identifiers`, the domain
  crates `api_community_content`, `api_identity_and_access`, `api_missions`, `api_operations` and
  `api_server_infrastructure`, `fleet_wire_contract`, sqlx, axum, serde, chrono, thiserror and
  uuid.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, and the integration
  suites call the leaderboard handler. Over HTTP: the dashboard in
  `crates/frontend/pages/command_center_pages/src/dashboard/` and the leaderboards in
  `crates/frontend/pages/operations_pages/src/leaderboards/`.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); its route table, its handlers
  and its imports follow the domain graph.

## Related documentation

- [API command center source](/crates/api/api_command_center/src/README.md) — the files, the
  routes and how the dashboard and the boards are read.
- [Match telemetry, fleet status and derived statistics](/documentation/crates/api/api_server/verification_evidence/telemetry.md)
  — the fleet block and when the derived statistics are recomputed.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
