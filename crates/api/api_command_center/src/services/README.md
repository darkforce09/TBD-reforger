# Command center services

The dashboard's overview of the configured game-server fleet. The derived figures the
leaderboards and statistics card read (each member's deployment count and attendance rate, and
the `leaderboard_totals` materialized view) are kept current by the member activity crate,
`crates/api/api_member_activity/src/`.

## Contents

```text
crates/api/api_command_center/src/services/
├── fleet_overview.rs  the active servers with their statuses, and the fleet and telemetry totals
├── mod.rs             the module tree
└── tests/             unit tests for the fleet totals
```

## How it works

- **Fleet overview.** `load_fleet_overview` reads the active servers ordered by name then id and
  their status rows through `SELECT_FLEET_STATUSES`, pairs each server with its status (none when
  it has no row), and `fleet_totals` counts the servers, the online ones and their players and
  capacity, and sums every reported telemetry queue's `backlog` and `dropped_total`.

## Boundaries

- Depends on: sqlx; `api_server_infrastructure` (`ServerStatus`, `ServerStatusRow`,
  `SELECT_FLEET_STATUSES`) for the fleet's statuses; the crate's `Error` for a failed read.
- Used by: the dashboard handler in `crates/api/api_command_center/src/handlers/`
  (`load_fleet_overview`); the dashboard suites in `crates/api/api_server/tests/` over HTTP.
- Rules: the fleet is exactly the active servers, so an inactive server never reaches the list or
  the totals (`crates/api/api_server/tests/fleet_and_ballistics/fleet_dashboard.rs`).
