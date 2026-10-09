# API crates

The library crates of the [API](/documentation/glossary/a_to_f.md#api): the code the API
application (`apps/api`) is built from, one crate per kernel concern or domain, each one free of
the application binary.

## Contents

```text
crates/api/
├── api_administration/      `api_administration`: the member roster and its moderation, the Discord role resync, the membership grace extension, the audit log console
├── api_audit_log/           `api_audit_log`: the audit severity and the best-effort and transactional audit line appends
├── api_background_workers/  `api_background_workers`: the interval tasks the API binary arms at boot, each calling a domain service
├── api_caller_identity/     `api_caller_identity`: the role ladder, session and account authority, the identity lock order, the machine caller
├── api_command_center/      `api_command_center`: the members' dashboard with its fleet overview, the community leaderboards, the per-player statistics card
├── api_community_content/   `api_community_content`: the announcements, the wiki, the vehicle database, modpacks, uploads and the equipment data viewer routes
├── api_configuration/       `api_configuration`: the environment configuration read at boot, trusted proxy networks, the process shutdown signal
├── api_database/            `api_database`: the Postgres pool, the embedded migrations, the development seeds, SQLSTATE predicates
├── api_discord/             `api_discord`: the Discord OAuth2, guild-member and announcement webhook clients and their typed failures
├── api_equipment_datasets/  `api_equipment_datasets`: the equipment dataset imports, their SQLite navigation index and the generation-pinned read queries
├── api_failpoints/          `api_failpoints`: the `fail_point!` macro and, in test builds, the failpoint catalogue and arming registry
├── api_foundation/          `api_foundation`: the handler error envelope, JSON wire formats, text policies, request parameters
├── api_http_layer/          `api_http_layer`: access tokens, the middleware chain and extractors, rate limiters, metrics and health, the realtime hub
├── api_identifiers/         `api_identifiers`: the serde- and sqlx-transparent typed ids of every API table key, Discord snowflake and game runtime key
├── api_identity_and_access/  `api_identity_and_access`: Discord sign-in, session tokens, the caller's profile, the Arma link handshake, Discord membership
├── api_match_telemetry/     `api_match_telemetry`: the game runtime's session-fenced heartbeat, match registration, results revisions and detailed event batches
├── api_member_activity/     `api_member_activity`: member statistics, the leaderboard refresh, attendance attribution, the re-evaluation queue
├── api_mission_vocabulary/  `api_mission_vocabulary`: the terrain and game mode enums several API domains name
├── api_missions/            `api_missions`: the mission library, versions, artifacts, reviews and approvals, deployments, the armory, factions and registries
├── api_operations/          `api_operations`: the event calendar and its access control, ORBAT slotting and reservations, service records, leave requests, fire missions, ballistics catalogs
├── api_property_evidence/   `api_property_evidence`: the dev-only property run recorder of the API's property tests
├── api_server_infrastructure/  `api_server_infrastructure`: the server registry, the live status feed, machine credentials, the fleet command ledger, runtime sessions
└── api_state/               `api_state`: the application state and its `FromRef` sub-state projections
```

## How it works

Every crate here declares `category = "crates/api"` in its `[package.metadata.layout]`, and the
category carries its own rules on top of those every library crate keeps:

- An API crate depends on foundation, contracts, mission, ballistics and API crates only; never on
  an application or a tool, and its dev-dependencies never on an application
  (`cargo xtask verify crate-tiers`).
- sqlx and axum appear only in API crates (`cargo xtask verify crate-tiers`, the firewall rules).
- No public `id` / `*_id` field and no public function parameter is a bare `Uuid`, `String`,
  `&str` or integer; ids come from `api_identifiers` (`cargo xtask verify crate-anatomy`).
- A crate whose public functions return a `Result` keeps a `thiserror` `Error` in `src/error.rs`
  and no anyhow; only the dev-only `failpoints` and `test_fixtures` features exist.

The tier of a crate is 1 plus the highest tier it depends on (0 with no workspace dependency), so
the domain crates sit above the kernel crates, and `api_mission_vocabulary` (tier 0) and
`api_identifiers` (tier 1) sit at the bottom of the category.

## Boundaries

- Depends on: the foundation, contracts, mission and ballistics crates, and external crates from
  the root `[workspace.dependencies]`.
- Used by: the API application (`apps/api`) and its integration tests.
- Rules: the category rules above, plus the anatomy every library crate keeps
  (`cargo xtask verify crate-anatomy`).
