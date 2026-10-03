# Command center handlers

The read surfaces of the command center: the members' dashboard, the ranked leaderboards and one
player's statistics card. Every handler takes `AuthUser` and presents figures other domains
produce.

## Contents

```text
apps/api/src/command_center/handlers/
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

- Depends on: `identity_and_access::services::user_lookup`,
  `missions::services::mission_lookup`, `community_content` (`load_current_modpack`,
  `Announcement`), `operations::models` (`Event`, `EventMission`, `OrbatSlot`) and the domain's
  `services::fleet_overview`; `core` for the extractor and errors.
- Used by: the domain's `routes.rs`; over HTTP, the dashboard in
  `apps/frontend/src/pages/command_center/dashboard/` and the leaderboards and operator
  dossier in `apps/frontend/src/pages/operations/leaderboards/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/api/src/tests/architecture_rules.rs`); a
  leaderboard category maps to its ordering only through the fixed list, never through request
  text.
