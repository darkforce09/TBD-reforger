# `command_center/`

The platform's read surfaces: the home dashboard that composes many best-effort lookups into one
response, the ranked community leaderboards, and a single player's aggregate statistics card.
Everything here presents figures that other domains produce; the ingest that creates them belongs
to `match_telemetry`, and the events they are attributed to belong to `operations`.

The domain owns no models of its own — it projects rows from the domains it reads.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by `core::http_router::api_v1_routes`
  and nested under `/api/v1`. The literals in `routes.rs` are the public URLs: `/dashboard`,
  `/leaderboards`, `/users/{discordId}/stats`.
- **`services::leaderboard_view::refresh_leaderboard`** — refreshes the `leaderboard_totals`
  materialized view; the scheduled refresher in `background_workers` calls it.
- **`services::user_stats`** — the denormalized `users.total_deployments` and
  `users.attendance_rate` recompute, called after attendance changes.

## Dependency rules

- Handlers here never import another domain's handlers. The dashboard reaches other domains through
  their services and models: `identity_and_access::services::user_lookup`,
  `missions::services::mission_lookup`, `community_content::services::modpack_lookup`,
  `community_content::models::announcement`, `operations::models`, and
  `server_infrastructure::models::server`.
- This domain imports `core`, the domains it reads, and `administration::services::audit_writer`
  for the papertrail its privileged reads leave. Nothing in `core` imports it.
- The materialized-view refresh has one home, `services/leaderboard_view.rs`, so the worker and the
  best-effort recompute after a write run the same statement.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for the command center.
handlers/
  mod.rs                               The read surfaces: leaderboards, dashboard, stats card.
  leaderboards.rs                      The ranked community leaderboard tables.
  live_dashboard.rs                    Home dashboard aggregation: best-effort, null-safe lookups in one response.
  user_stats_card.rs                   One player's aggregate statistics card.
  tests/
    leaderboards.rs                    Sibling unit tests for `leaderboards.rs`.
services/
  mod.rs                               Business logic behind the command center surfaces.
  leaderboard_view.rs                  Refresh of the `leaderboard_totals` materialized view.
  user_stats.rs                        Denormalized per-user deployment count and attendance rate.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. The dashboard and leaderboard reads are
covered end to end by the integration suites under the crate's `tests/` directory.
