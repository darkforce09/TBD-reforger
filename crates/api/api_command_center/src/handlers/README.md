# Command center handlers

The read surfaces of the command center: the members' dashboard, the ranked leaderboards and one
player's statistics card. Every handler takes `AuthUser` and presents figures other domains
produce.

## Contents

```text
crates/api/api_command_center/src/handlers/
├── leaderboards.rs     the ranked leaderboard for one category, searchable by name
├── live_dashboard.rs   the dashboard: best-effort, null-safe lookups composed into one answer
├── mod.rs              the module tree
├── tests/              unit tests for the leaderboard categories and ordering
└── user_stats_card.rs  one player's aggregate statistics card
```

## How it works

`GET /api/v1/leaderboards` reads the `leaderboard_totals` materialized view. `?category=` picks the
ranking (`kd`, the default, `command_win`, `missions`, `longest_kill` or `team_kills`) from a fixed
list, any other value is a 400, and every ordering ends on `discord_id` so tied scores page
deterministically. `GET /api/v1/users/{discordId}/stats` reads the same projection for one account.
`GET /api/v1/dashboard` composes the next [event](/documentation/glossary/a_to_f.md#event), the caller's
assignment in it, the configured fleet (`fleet`: every active server with its status, and the
fleet totals, from `services::fleet_overview`), the current modpack and the latest announcements;
each lookup is best-effort, so a missing piece is `null` rather than a failed dashboard.

## Boundaries

- Depends on: `api_identity_and_access::services::user_lookup`,
  `api_missions::services::mission_lookup`, `api_community_content` (`load_current_modpack`,
  `Announcement`), `api_operations::models` (`Event`, `EventMission`, `OrbatSlot`) and the domain's
  `services::fleet_overview`; `api_http_layer` for the extractor, `api_foundation` for the errors
  and `api_state` for the application state.
- Used by: the domain's `routes.rs`; the leaderboard paging suite in `crates/api/api_server/tests/`; over HTTP, the dashboard in
  `crates/frontend/pages/command_center_pages/src/dashboard/` and the leaderboards and operator
  dossier in `crates/frontend/pages/operations_pages/src/leaderboards/`.
- Rules: every handler carries its `/// @route` tag; no handler
  imports another domain's handlers; a
  leaderboard category maps to its ordering only through the fixed list, never through request
  text.
