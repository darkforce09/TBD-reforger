**Status:** archived

# API V2 Phase Two Handoff

Status: `core/`, `background_workers/`, and the eight domain route tables are live on `main` (commits `8fc8dca30`, `67c016ccf`, `da0c23bbb`, `d076fba1a`, `ece65ae6e`). Handlers, services, models, and the contract module still sit in their legacy directories; Phases Three to Five move them domain by domain.

## Ownership and layout

- `src/core/` owns the composition root and every crate-wide primitive: `http_router.rs` (middleware chain, `/healthz`, `/metrics`, the `/api/v1` merge of the eight domain tables), `application_state.rs`, `configuration/`, `database/` (pool, connect, migrate, Postgres error predicates, and the leaderboard materialized-view refresh statement until command_center lands), `error_handling/`, `observability/`, `middleware/` (authentication extractors, CORS, request correlation, in-memory and durable rate limiting, client identity), `realtime_hub/` (topic hub, and the server-status topic publishers until server_infrastructure lands), `authentication_primitives/`, `http/pagination.rs`, `http_client/retry_on_429.rs`, `text/`, `wire_format/`.
- `src/background_workers/` owns the six tickers and `spawn_all`, which `bin/api.rs` calls once. Pure query functions the tickers call (`purge_expired_refresh_tokens`, `resync_all_roles`, `sweep_once`, `refresh_leaderboard`, `publish_all_server_statuses`) remain with their current modules until their domains land.
- `src/<domain>/routes.rs` (eight files) register the 78 `/api/v1` routes with the same path literals and handler expressions as before; `core/http_router.rs::api_v1_routes` merges them. Every public URL is unchanged.
- Legacy `src/{handlers, services, models, contract}` remain and are referenced from the route tables as `crate::handlers::…`.

## Gate and pin changes

- `tools_v2/xtask/src/verifications/architecture/route_tags{.rs, /verify_route_tags.rs, /route_and_tag_extraction.rs}`: the gate discovers `src/<domain>/routes.rs`, parses each file's single `pub fn routes`, cross-checks the `.merge(crate::<domain>::routes(` list in `api_v1_routes` against the discovered files in both directions, and keeps its vacuity guards, four sentinels (spanning three files), collation and A/B cross-check. 14 tests.
- Frontend `core/test_support/fixtures.rs::api_route_source()` concatenates the eight route tables; four page test files assert against it.
- `tests/null_tolerance.rs` sweeps the eight tables plus `core/http_router.rs` (for `/healthz` and `/metrics`).
- Source-text tests in `middleware` and the handler files read `core/http_router.rs`, `core/application_state.rs`, or the owning domain's `routes.rs`.
- Allowlist: `app.rs`, `config.rs`, `db.rs`, `middleware/ratelimit.rs` exemptions retired; nothing added.

## Verification evidence

Logs under the session scratchpad `phase2/` (`2_1_*` to `2_6_*`).

- Every sub-phase: `cargo check -p website-api --all-targets`, `cargo fmt --all --check`, `cargo xtask verify file-length` (0 violations), `cargo xtask verify route-tags` (PASS, 103 tags against 103 routes, route rows identical to the Phase 0 golden) — all green.
- `cargo test -p website-api --lib --bins`: 281 → 282 (one router-merge test added).
- `cargo test -p xtask -p ticket-engine`: 845 passed, 0 failed.
- `cargo xtask db test-it`: 44 targets, 785 passed, 0 failed, 1 ignored.
- Boot smoke (2.4): six workers armed, `/healthz` 200.
- `cargo xtask ci ci-local`: pass (rc 0), log `phase2/2_6_ci-local.log`.

## Working-tree handling

- The other session's ten tracked-but-deleted paths are staged only for the duration of a `ci ci-local` run and restored with `git reset -- <paths>` afterwards; no Phase 2 commit contains them.
- The local development database (`tbd_reforger_db`) carries an older checksum for migration `0021`, so a bare boot of the binary against it needs `SKIP_MIGRATE=1`. Integration suites create their own databases and are unaffected. Not caused by this refactor; resolve by recreating the dev database when convenient.

## Temporary placements (tracked)

- `core/database/leaderboard_refresh.rs` → `command_center/services/leaderboard_view.rs` in Phase 5.
- `core/realtime_hub/server_status_topic.rs` → `server_infrastructure/services/status_broadcast.rs` in Phase 3.
- `background_workers/event_lifecycle_sweeper.rs` calls `handlers::events::sweep_once` until Phase 4 moves it to `operations/services/event_lifecycle_sweep.rs`.
- `handlers/mod.rs` still hosts `load_user`, `load_mission`, `username` until Phases 3 and 4.
- Scaffold READMEs under `src/core/**` and the domain directories still describe the blueprint's planned filenames; Phase 6 rewrites them.
