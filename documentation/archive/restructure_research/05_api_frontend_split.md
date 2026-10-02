**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# API and frontend split analysis

I've finished the read-only analysis of both crates. In short, neither splits cleanly yet. The API has 9 two-way dependencies between its domains, nearly all caused by about 1.3k lines of shared helper services, plus an `AppState` (the shared handler state) that holds services owned by two domains. In the frontend the bottom-layer `core` modules depend on each other in a loop, `core` reaches up into the editor in several places, and the 45k-line editor is one tangle that can't be split internally.

How the numbers were measured: "prod" excludes `*/tests/*`; generated (typify) code is shown separately. Import counts are `crate::<x>::` path occurrences with doc comments included; test files are excluded only in the frontend matrix and the pairs listed below.

# A. `apps/website/api_v2` (website-api)

Edition 2024. One lib plus 3 bins (`Cargo.toml:12-26`). There is one feature, `failpoints`, enabled only by a dev-dependency on the crate itself (`Cargo.toml` `[dev-dependencies]` `website-api = { path=".", features=["failpoints"] }`). `src/` is 82.3k lines and `tests/` is 89.6k.

## A1. Sizes and internal layering

Every domain has the same layout: `mod.rs`, `routes.rs` with `pub fn routes(..) -> Router<AppState>`, `handlers/`, `services/`, `models/` (with `models/generated/`), and sibling `tests/` folders. There is no `repositories/` layer anywhere; SQL lives in `services/`. sqlx uses runtime queries only (536 `sqlx::query(..)`-style calls, 0 `query!` macros) and a single `migrations/` folder (`core/database/mod.rs:76`). That means no `.sqlx` offline cache is needed per crate.

| Domain | Prod (excl. gen) | handlers / services / models | Generated | Test |
|---|---|---|---|---|
| operations | 11,493 | 3795 / 6428 / 1101 | 4,024 | 2,973 |
| community_content | 7,606 | 3025 / 4029 / 392 | 3,485 | 2,399 |
| missions | 7,047 | 2961 / 2398 / 575, plus `contract/` 600 and `validation/` 352 | 6,580 (contract 2,334 + models 4,246) | 2,372 |
| server_infrastructure | 3,638 | 1229 / 1757 / 550 | 1,520 | 430 |
| identity_and_access | 3,340 | 931 / 2185 / 165 | 161 | 1,302 |
| match_telemetry | 2,465 | 511 / 889 / 1022 | 2,244 | 760 |
| administration | 1,871 | 816 / 806 / 190 | 658 | 530 |
| command_center | 639 | 385 / 223 / – | 0 | 115 |
| background_workers | 822 | 14 worker files | – | 316 |
| bin/ | 3,825 (staging_fixtures 4,272 incl. tests) | – | – | 614 |

`core/` is 4,476 prod / 3,676 test lines:

| core submodule | Prod | Test |
|---|---|---|
| middleware | 925 | 730 |
| observability | 831 | 263 |
| failpoints | 544 | 343 |
| configuration | 518 | 475 |
| http_router.rs | 252 | – |
| database | 254 | 166 |
| text | 254 | 501 |
| error_handling | 175 | 218 |
| authentication_primitives | 158 | 248 |
| wire_format | 141 | 27 |
| application_state.rs | 132 | – |
| process_lifecycle | 78 | 93 |
| http | 75 | 108 |
| realtime_hub | 60 | 25 |
| http_client | 56 | 13 |

There are also `core/tests` (466) and `src/tests/` (882 lines of architecture and prose rules).

## A2. Import matrix (rows import columns)

| from \ to | admin | cmdctr | commun | IAM | telem | missions | ops | infra | workers | core |
|---|---|---|---|---|---|---|---|---|---|---|
| administration | – | 0 | 0 | 5 | 0 | 0 | **1** | 0 | 0 | 24 |
| command_center | 2 | – | 2 | 1 | 0 | 1 | 1 | 3 | 0 | 13 |
| community_content | 11 | 0 | – | 0 | 0 | 0 | 0 | 0 | 0 | 74 |
| identity_and_access | 6 | **2** | 0 | – | 0 | 0 | **2** | **2** | 1 (doc) | 40 |
| match_telemetry | 3 | 2 | 0 | 1 | – | 2 | 1 | 14 | 0 | 37 |
| missions | 9 | 0 | 1 | 5 | 0 | – | **3** | 4 | 0 | 92 |
| operations | 13 | **2** | 0 | 12 | **1** | 6 | – | 6 | 0 | 118 |
| server_infrastructure | 9 | 0 | 2 | 5 | 0 | **1** | 0 | – | 0 | 52 |
| background_workers | 1 | 1 | 1 | 3 | 0 | 1 | 5 | 6 | – | 12 |
| core | 1 | 1 | 4 | 3 | 1 | 1 | 1 | 1 | 2 (doc) | – |

**Cycles between domains (9 pairs). The bold cell is the edge that closes each loop:**
- **administration ↔ identity_and_access.** `administration/handlers/{membership_grace_overrides.rs:10, role_management.rs:17-18, disciplinary.rs:23}`, `models/personnel_page.rs:16`. The other direction is identity_and_access calling `administration::services::required_audit` (6).
- **administration ↔ operations.** `administration/handlers/disciplinary.rs:24` calls `operations::…::reevaluation_queue::request_reevaluation_for_account`; operations calls `required_audit` (13).
- **identity_and_access ↔ operations.** `identity_and_access/services/discord_membership_cache.rs:5` and `identity_linking.rs:122` (`participation_attribution::refresh_attendance`).
- **identity_and_access ↔ command_center.** `identity_and_access/services/user_lookup.rs:6` (`ATTENDANCE_RATE_SQL`) and `identity_linking.rs:7` (user_stats); command_center calls `user_lookup`.
- **identity_and_access ↔ server_infrastructure.** `identity_and_access/handlers/arma_link_confirmation.rs:9-10` (`ExecutorKind`, `MachineCaller`); server_infrastructure calls `session_authorization`.
- **missions ↔ operations.** `missions/services/mission_deployments/deployment_selection.rs:12` (`parse_orbat_template`) and `slot_bindings.rs:15` (`OrbatSquadTemplate`).
- **missions ↔ server_infrastructure.** `server_infrastructure/handlers/server_intel.rs:28` (`missions::models::mission::TerrainType`), against `missions/services/mission_deployments/deployment_requests.rs:22-23` (fleet_commands).
- **operations ↔ command_center.** `operations/services/event_reservations/{event_administration.rs:143, mission_restoration.rs:76}` call `user_stats::recompute_user_stats_on_connection`, against `command_center/handlers/live_dashboard.rs:21`.
- **operations ↔ match_telemetry.** `operations/handlers/member_service_record.rs:50` (`Match`, `MatchPlayerStat`), against `match_telemetry/services/match_results_ingest.rs:32`.

**Small shared services cause almost every cycle:**
- **Audit:** `administration/services/required_audit.rs` (81), `audit_writer.rs` (91) and `models/audit_log.rs` (62). They depend only on `core::wire_format`. Used by 6 domains.
- **Stats:** `command_center/services/user_stats.rs` (85) depends only on audit and core.
- **Machine identity:** `server_infrastructure/services/machine_credentials.rs` (237), `models/machine_credential.rs` (87) and `machine_authentication.rs` (30, which holds `MachineCaller`'s `FromRequestParts<AppState>` impl).
- **Session and identity:** `identity_and_access/services/session_authorization.rs` (94) and `identity_ownership.rs` (39).
- **Operations helpers:** `operations/services/participation_attribution.rs` (114), `event_reservations/reevaluation_queue.rs` (152), and the ORBAT template parser.
- **Mission model:** `missions/models/mission.rs` (219, including `TerrainType`).

Together these are about 1.3k lines. Moving them into one or two lower "shared-kernel" crates removes every cycle except missions ↔ operations (ORBAT template) and missions ↔ server_infrastructure (fleet commands vs `TerrainType`). Those two are fixed by moving `OrbatSquadTemplate`/`parse_orbat_template` and `TerrainType` into the shared kernel.

**Domain → core submodule** (order: app_state / err / http / middleware / wire / text / authprim / failpoints / config / db):

| Domain | app_state | err | http | middleware | wire | text | authprim | failpoints | config | db |
|---|---|---|---|---|---|---|---|---|---|---|
| operations | 20 | 42 | 18 | 18 | 13 | 1 | 0 | 1 | 2 | 3 |
| missions | 15 | 27 | 13 | 16 | 13 | 1 | 0 | 4 | 2 | 1 |
| community_content | 18 | 16 | 13 | 14 | 4 | 8 | 0 | 0 | 0 | 0 |
| server_infrastructure | 10 | 17 | 8 | 7 | 6 | 0 | 1 | 2 | 0 | 0 |
| match_telemetry | 9 | 16 | 2 | 1 | 6 | 1 | 0 | 1 | 0 | 1 |
| identity_and_access | 9 | 8 | 0 | 3 | 4 | 1 | 6 | 2 | 2 | 0 |
| administration | 5 | 4 | 3 | 5 | 4 | 0 | 0 | 2 | 0 | 0 |
| command_center | 4 | 4 | 1 | 3 | 1 | 0 | 0 | 0 | 0 | 0 |

`realtime_hub` is used directly only by server_infrastructure (1) and workers (1). `http_router`, `observability` and `process_lifecycle` are used only by bins.

## A3. AppState and router

- `AppState` (`core/application_state.rs:25-45`) holds `pool`, `cfg`, `jwt`, `session_authority: Arc<dyn SessionAuthority>`, `cors_origins`, two rate limiters, `hub`, `metrics_registry`, and three domain-owned services:
  - `equipment_data` (from community_content, `:26-27`)
  - `discord: Arc<DiscordService>` (from identity_and_access, `:20`, 305 lines)
  - `webhook: Arc<WebhookService>` (from community_content, `:13`, 169 lines; it imports `community_content::models::announcement`)
- `AppState::new` (`:50-91`) builds `DatabaseSessionAuthority`, which comes from identity_and_access (`:72-75`).
- `FromRef` impls exist for the sub-states (`:94-132`), but handlers almost never use them: there are 168 `State<AppState>` extractors and 2 `State<PgPool>`.
- Authentication already avoids depending on domains: `core/middleware/authentication.rs:22-32,64-70` defines `AuthUser { role: String }` and the role-gate newtypes over the `SessionAuthority` trait (`core/authentication_primitives/session_authority.rs:7`). `MachineCaller` implements `FromRequestParts<AppState>` concretely (`server_infrastructure/services/machine_authentication.rs:17`).
- The router `api_v1_routes` (`core/http_router.rs:37-47`) just merges the 8 domain `routes()` functions. Three take parameters: `identity_and_access::routes(dev)`, `missions::routes(version_limit)`, `community_content::routes(dev)`. `router(state)` (`:63+`) adds `/healthz`, `/metrics`, `/uploads`, `/map-assets` and the middleware chain.
- The realtime hub (`core/realtime_hub/mod.rs`) is a topic-string → `Vec<u8>` broadcast with no domain types. Its only publisher is `server_infrastructure/services/status_broadcast.rs:32` and its only subscriber is `server_status_stream.rs:50`. It does not block a split.
- `src/tests/architecture_rules.rs:258-259` already enforces "core imports no domain except `application_state.rs` and `http_router.rs`". It also checks no-foreign-handlers (`:284`), workers used only by the binary (`:313`), and one route table per domain (`:340`).

**Verdict: `fn routes() -> Router<AppState>` per domain crate is feasible.** What blocks it today:
1. `AppState` holds the three domain services. Fix: move `DiscordService` and `WebhookService` (plus the announcement fields it needs) into a low `api_discord` crate. Put `EquipmentDatasets` (about 2.5k lines) behind an `Arc<dyn …>` or its own crate. Have `AppState::new` take `Arc<dyn SessionAuthority>` so the app crate builds it.
2. The 9 domain cycles above.
3. `MachineCaller` is bound to the concrete `AppState`. That is fine once `AppState` lives in an `api_state` crate.
4. `fail_point!` is `#[macro_export]` and uses `$crate::core::failpoints` (`core/failpoints/mod.rs:31-50`). It must move to a `api_failpoints` crate. The `Failpoint` catalogue enum is central, so core knows every domain's failpoint names. Domain usage: missions 4, identity_and_access 3, administration 2, server_infrastructure 2, telemetry 1, operations 1.
5. 152 `pub(crate)` items, some of which are used across domains.

## A4. Errors

- There is one `ApiError` struct `{status, message, details}` (`core/error_handling/api_error.rs:36`), with `IntoResponse` (`:149`) and `From<sqlx::Error>` (`:163`).
- References per domain: operations 410, missions 234, server_infrastructure 182, community_content 152, identity_and_access 93, telemetry 72, administration 42, command_center 14.
- Other error types: `ImportError` (`missions/services/registry_import.rs:45`), `ContractError` (`missions/contract/schema_validators.rs:35`), `CatalogLoadError` (`operations/services/ballistics_catalogs/catalog_store.rs:240`), `ConfigError` (`core/configuration/mod.rs:109`), `ToolFailure` (`bin/staging_fixtures/tool_failure.rs:28`).
- `anyhow` appears 58 times, concentrated in `community_content/services/equipment_data_viewer/**` (about 18), bins (2), identity_and_access (4) and failpoints (1).

## A5. Generated code, schema validators, contracts

- 266 typify files, 18,672 lines, generated by `tools_v2/xtask/src/commands/generate/schema_types.rs:23` (output root `API_SOURCE_DIR = "apps/website/api_v2/src"`).
- Per domain: missions 6,580 (`contract/generated` 2,334 + `models/generated` 4,246), operations 4,024, community_content 3,485, match_telemetry 2,244, server_infrastructure 1,520, administration 658, identity_and_access 161.
- These are almost all used by tests: 19 integration-test files use them, but only 6 production references exist (`missions/services/registry_import.rs:18`, `community_content/handlers/equipment_data_viewer/{dataset.rs:4, resources.rs:4, source_inspection.rs:4,104}`, `media_upload/mod.rs:36`). They are a good candidate for a separate `api_contract_types` crate.
- `missions/contract/` holds `schema_validators.rs` (embeds `contracts_v2/definitions/*.schema.json` via `include_str!("../../../../../../contracts_v2/…")` at `:20-30`; these relative paths must be rewritten on a move), `zone_quantisation.rs` and `loadout_projection.rs`. It depends on `website_map_engine::data::scenario::wire_safety` (`:16`).

## A6. Binaries

- `api.rs` (86 lines) imports `background_workers`, `core::{application_state, configuration, process_lifecycle, database, http_router}` (`:18-22`).
- `import_registry.rs` (81) imports `core::database` and `missions::services::registry_import` (`:13-14`).
- `staging_fixtures/` (about 4.3k) imports:
  - administration `required_audit` (4 files)
  - identity_and_access `account_authority`, `discord_client`, `account_registration`, `discord_membership_cache`, `session_issuance` (`load_population/population_seeding.rs:27-35`)
  - operations `event_authoring` and the ORBAT templates (`load_fixture_events/fixture_plan.rs:26-30`)
  - server_infrastructure `machine_credentials`, `server_registration`, `ExecutorKind` (`fleet_provisioning.rs:24-26`, `credential_rotation.rs:25-27`)
  - core `ApiError`, `hash_token`, `database`

## A7. Integration tests

- 154 top-level binaries, about 65k lines; support directories about 24.7k.
- Largest support directories: `route_acceptance_support` 10,850, `contract_parity_support` 2,199, `null_tolerance_support` 1,872, `common` 1,500 (`database.rs`, `http.rs`, `fixtures.rs`, `source_text.rs`, plus `#[path="../../src/tests/property_evidence.rs"]`). 146 binaries declare `mod common;`.
- Most tests are black-box. Imports across all tests: `core::database` 49, `application_state` 38, `configuration` 36, `http_router` 33. 90 of 154 binaries import no domain module at all and go through the router.
- Of the 64 that do, 54 touch one domain, 9 touch two and 1 touches three. By domain: identity_and_access 22 files, operations 12, administration 9, server_infrastructure 9, background_workers 6, community_content 5, missions 5, telemetry 4, command_center 3.
- By URL segment, the dominant targets are `/ingest`, `/me`, `/auth`, `/events`, `/event-missions`, `/missions`, `/game-runtime`, `/servers`, `/fleet-executor`.
- **Consequence:** almost all suites need the composed router, so they belong to the top-level app crate, or to a dedicated `website_api_it` test crate. `tests/engineering_laws.rs` (218) is repository-wide; it uses `verification_core` with `WEBSITE_API_RULE`/`FRONTEND_RULE` and needs updating for new crate names.

## A8. `apps/fleet_host_agent` (7.7k)

- Edition 2024, no dependency on website-api.
- Prod lines: `rcon` 928, `ledger_client` 581, `command_execution` 579, `process_control` 454, `agent_configuration` 441, `dedicated_server_config` 383, root about 253. Tests are 2,136 integration (fake BattlEye server, fake ledger API, fake systemctl) plus about 2k unit; 19 `#[path]`.
- **The fleet wire types exist in four copies:**
  1. API hand-written: `server_infrastructure/models/fleet_command.rs:40` (`FleetAction`), `:242` (`ClaimedFleetCommand`), `:255` (`ExecutionStart`), `:262` (`ExecutionResult`).
  2. API typify: `models/generated/fleet_command/{claimed_fleet_command, execution_result, execution_start, fleet_action, claim_request}.rs`.
  3. Agent: `ledger_client/ledger_messages.rs:13,26,33`, plus its own copy of the API error shape `ErrorEnvelope` (`:56`); actions parsed from strings in `command_execution/host_command.rs:26`.
  4. Frontend: `core/api/dto/fleet_commands.rs` (183).
- The machine-credential secret shape is also duplicated: agent `agent_configuration/secret_files.rs:17` vs `bin/staging_fixtures/secret_files.rs`.
- All of these cite `contracts_v2/definitions/fleet-command.schema.json`, but no code is shared. A serde-only `contracts_fleet` crate could serve all three.

## A — proposed crates

| Crate | Contents | ~Prod lines | Blocker to resolve |
|---|---|---|---|
| `api_error` | error_handling + http/path_parameters | 250 | none |
| `api_wire` | wire_format + text | 400 | none |
| `api_config` | configuration + process_lifecycle | 600 | none |
| `api_db` | database (owns `migrations/`) | 250 | `sqlx::migrate!` path |
| `api_failpoints` | failpoints | 550 | `$crate` path; catalogue knows domain names |
| `api_http` | authentication_primitives, middleware, observability, realtime_hub, http_client | 2.1k | `durable_ratelimit` doc-links workers (doc only) |
| `api_discord` | `DiscordService`, `WebhookService` | 500 | webhook imports `community_content::models::announcement` |
| `api_state` | `AppState` + `FromRef` | 150 | constructor must take `Arc<dyn SessionAuthority>`; equipment datasets out |
| `api_kernel` | audit, user_stats, machine credentials/`MachineCaller`, session_authorization, identity_ownership, participation_attribution, reevaluation_queue, ORBAT templates, `TerrainType`/mission model | 1.3–1.8k | the 9 cycles |
| `api_contract_types` | all typify output, optionally per domain | 18.7k | xtask output path; 6 prod refs |
| `api_<domain>` × 8 | as A1, each with `routes()` | 0.6k–11.5k | `pub(crate)` → `pub` |
| `api_workers` | background_workers | 0.8k | none |
| `website_api` (app + 3 bins) | http_router, bins | 0.5k + 3.8k bins | — |
| `website_api_it` | the 154 integration suites | 89.6k | needs every crate |

Operations (11.5k) could later split into `api_ops_events`, `api_ops_reservations` and `api_ops_ballistics` along `services/{event_authoring, event_reservations, ballistics_catalogs, event_access, access_administration}`.

# B. `apps/website/frontend` (website-frontend)

Edition **2021** (the API is 2024). It is a bin crate (`main.rs`) with no `[features]`. The map-engine dependency is target-split: native gets `world, io, store, editing`; wasm32 gets `render, streaming`. Total 166.5k lines. There are 298 `#[path]` attributes and 420 `#[cfg(test)]`, with tests in sibling `tests/` folders.

## B1. Sizes (prod / test)

| core module | Prod | Test |
|---|---|---|
| api | 6,760 (dto 3,444, client 1,428, endpoints 927, sse+frames 411, audit_stream 237) | 6,657 |
| auth | 1,140 | 1,303 |
| map_view | 914 | 206 |
| offline | 1,400 | 586 |
| ui | 1,461 | 938 |
| utils | 564 | 423 |
| test_support | 0 | 2,302 |

| Page area | Prod | Test | Main parts |
|---|---|---|---|
| administration | 15,033 | 4,811 | event_manager 5,387; server_control 4,628; personnel 1,394; approvals, content_manager, ballistics_catalogs, audit_logs about 0.8–1k each |
| mission_hub | 5,662 | 1,911 | library 2,888 |
| operations | 4,962 | 1,406 | event_detail 2,701 |
| doctrine_and_info | 4,732 | 2,674 | wiki 2,464 |
| field_tools/mortar | 4,033 | 2,152 | – |
| command_center | 1,593 | 186 | – |
| navigation | 909 | 126 | – |
| account | 613 | 31 | – |

- apps/editor: 45,050 / 37,056. Breakdown in B5.
- apps/debug: 5,972 / 1,629.
- `apps/aar` and `apps/planner` contain only a README.
- Root files: `router.rs` 377, `app_routes.rs` 94, `main.rs` 44.

## B2. Import matrix (prod only, `crate::v2::` occurrences)

- **core:**
  - api → auth 23, auth → api 2 (a **cycle**: `core/auth/store.rs:24` uses `dto::MeResponse`; `session.rs:197` uses `client::refresh::with_refresh_lock`; the client uses `AuthStore`, `RefreshResponse` and `SingleFlight`).
  - auth → ui 1 and ui → auth 2 (a **cycle**: `core/auth/route_guard.rs:55` uses `Toasts`; `core/ui/gates.rs:18-19` uses `AuthStore` and `Role`).
  - utils → ui 1 (`core/utils/clipboard.rs:17`).
  - offline → api 2.
- **pages → core** (api / auth / ui / utils / map_view / offline): administration 173/27/81/20; operations 56/22/37/18; mission_hub 51/18/37/6; doctrine_and_info 68/18/26/5; field_tools 16/2/2/1 plus map_view 12 and offline 14; command_center 19/4/16/6; account 9/5/3/1; navigation 10/7/5/1.
- **apps → core:** debug → api 19 only. Editor → core: ui sub 57 api / 51 ui / 10 auth; shell 14/10/3; mission_editor 11 api / 12 map_view / 7 auth; bridge 9 ui / 3 map_view / 3 api; arsenal 5 api. No app imports `pages`.

**Every dependency that points the wrong way:**
1. **core → apps**
   - `core/ui/slider.rs:19`, `select.rs:15`, `search_box.rs:18` import `apps::editor::shell::layout::{DISABLED_GLYPH, HOVER_FILL}`.
   - `core/auth/store.rs:259` calls `apps::editor::shell::hydrate::purge_local_documents`.
2. **core → crate root**
   - `core/auth/route_guard.rs:28,31` calls `crate::router::role_may_enter` and `auth_denial_redirect`.
   - `router.rs:13` imports `core::auth` back, which is another cycle.
3. **pages → apps** (only mission_hub)
   - `pages/mission_hub/review_workspace/page.rs:17-18` (`MissionEditorPage`, `shell::review_mode`).
   - `library/dossier_upload.rs:35-36` and `dossier_upload_panel.rs:311` (`shell::mission_size::format_bytes`).
   - Tests: `review_workspace/tests/review_workspace.rs:6,104,108` (`tab_lock`).
4. **page area → another page area**
   - `administration/approvals/{review_drawer.rs:27-32, submission_queue.rs:22, review_decision.rs:17}` and `administration/server_control/mission_deployments/deployment_wording.rs:21` use `mission_hub::mission_review::*` (7 uses).
   - Tests: `administration/ballistics_catalogs/tests/ballistics_catalogs.rs:325` uses `navigation::nav_config`.
5. **navigation → every page**
   - `pages/navigation/layout.rs:26` imports `crate::app_routes::AppRoutes`, which references every page component (`app_routes.rs:13-33` plus inline paths `:41-89`).
   - `top_nav.rs:125` and `layout.rs:65,109` use `crate::router::{chromeless, full_bleed, breadcrumb}`.
6. **test-support source pins**
   - `core/test_support/pins.rs:265+` uses `include_str!("../../pages/administration/…")` and similar across page areas.
   - Cross-folder `include_str!` pins appear in editor (10 files), administration (2) and field_tools (1).

**Editor internals are one tangle with no clean layering:**

| from → to | ui | bridge | shell | arsenal | mission_editor | input |
|---|---|---|---|---|---|---|
| ui | – | 58 | 40 | 42 | 8 | – |
| bridge | 15 | – | 7 | 10 | 8 | 3 |
| shell | 17 | 20 | – | – | 4 | – |
| arsenal | 1 | 4 | 1 | – | – | – |
| input | 3 | 19 | 8 | – | 3 | – |
| mission_editor | 5 | 4 | 9 | 13 | – | 13 |

Specific back-edges:
- `arsenal/mod.rs:55` imports `ui::arsenal::panels`.
- `shell/eden_chrome.rs:16-30` re-exports ui docks and modals.
- `shell/layout.rs:79` takes `DOCK_BOTTOM_PX` from `ui::docks::toolbelt`.
- `bridge/host_state/**` imports `ui::outliner` and `ui::inspector`, e.g. `editor_context/mod.rs:44-45` and `armed_placement/zone_draw.rs:19`.
- `bridge/world_assets.rs:15` implements `ui::inspector::validation_panel::SeamRegistration`.

## B3. Map-engine use, routing, contexts

- **Map-engine users (prod files):**
  - `core/map_view` (10; `streaming::{host, bridge}`, `world::terrain`, `frame`)
  - `core/api/dto` (5; it re-exports `data::scenario::ballistics` and `data::store::operations::{faction_library, environment}` at `dto/fire_missions.rs:37`, `dto/registry.rs:18-20`, `dto/missions.rs:21,197`, `dto/ballistics_catalogs.rs:22`, `dto/mission_reviews.rs:142`)
  - `pages/field_tools` (16; scenario::ballistics, `camera::grid_reference`, `overlay`, `spatial::los`)
  - `pages/mission_hub` (1; `library/dossier_upload_panel.rs:211` `compile::version_body_to_writer`)
  - `apps/debug` (10; `world::architecture` 50)
  - editor (about 100 files)
- `core/offline` uses `website_offline_service_worker` (5 files).
- **Pages that only use transport, DTOs, UI and auth (no map-engine):** account, administration, command_center, doctrine_and_info, navigation, operations. These are clean page-crate candidates. A DTO crate still pulls map-engine with `scenario` + `store` (yrs) unless those re-exports are swapped for wire mirrors.
- **Routing:** `router.rs` is a data table (paths, auth tier, `full_bleed`, `chromeless`, breadcrumb) and is used by `core/auth`. `app_routes.rs` names every page component directly. Splitting pages into crates is fine as long as `app_routes`, `AppLayout` (navigation) and `main.rs` stay in the top-level bin crate.
- **Global contexts:**
  - `AuthStore` is provided at `pages/navigation/layout.rs:80`; about 19 consumers, including editor `ui/docks/dock_right/shell/layout.rs:87` and `compositions/mod.rs:117`.
  - `Toasts` is provided at `core/ui/toast.rs:91`; consumers include `core/auth/route_guard.rs:55` and editor `shell/save_status.rs:107,224` and `hydrate.rs:48`.
  - `ViewerContext` and `GridContext` are local to debug.
  - `AuthStore` and `Toasts` must live in shared crates.
- **thread_local stores:** editor 27 files (ui 10, shell 9, bridge 5, input 2, mission_editor 1), core/api 4, offline 2, ui 1, mortar 1, debug 2. Each is crate-private state; no cross-crate globals were found beyond these.

## B4. DTOs

`core/api/dto` is 3,444 lines in 21 files. `dto/mod.rs` glob re-exports most files (`pub use *::*`), which hides the edges, so this mapping was done by type name.

| DTO file | Lines | Consumers |
|---|---|---|
| event_access_administration | 319 | administration only |
| wiki | 301 | doctrine_and_info only |
| servers | 286 | administration, command_center |
| events | 279 | operations (18), administration (5) |
| missions | 231 | mission_hub, administration, editor (4), operations |
| fire_missions | 226 | field_tools only |
| mission_reviews | 197 | mission_hub, administration, editor |
| fleet_commands | 183 | administration (other hits are fn-name noise) |
| registry | 170 | editor only (26) |
| match_events | 141 | no page consumer found (core/sse) |
| administration | 135 | administration |
| telemetry | 106 | command_center, operations |
| content | 97 | administration, command_center, doctrine_and_info, mission_hub |
| vehicles | 96 | doctrine_and_info, editor |
| ballistics_catalogs | 83 | administration, field_tools, debug |
| mission_deployments | 78 | administration |
| common | 73 | most areas |
| auth | 72 | core, account, administration, navigation, operations |
| fleet_scenarios | 40 | administration |
| equipment_data_viewer/ | 216 | debug only |

The DTOs are hand-written and not shared with the API's typify types.

## B5. Editor

| Folder | Prod | Test |
|---|---|---|
| ui | 24,224 | 18,453 |
| ↳ docks | 9,084 (dock_right 3,596, top_strip 2,122, dock_left 1,397, context_menu 930, toolbelt 577) | 7,948 |
| ↳ inspector | 8,047 | 4,434 |
| ↳ modals | 4,054 | 4,234 |
| ↳ outliner | 2,437 | 1,770 |
| ↳ arsenal (UI panels) | 573 | 67 |
| arsenal (domain) | 5,265 | 5,294 |
| shell | 5,314 | 3,157 |
| bridge | 4,587 | 621 |
| input | 3,132 | 293 |
| mission_editor | 2,020, plus `mission_editor.rs` 469 | – |
| editor/tests | – | 9,238 |

The README describes the intended layering (`apps/editor/README.md`), but the code does not follow it (B2). Debug benches: building_viewer 1,968, data_viewer 1,874, world_los 625 (+ `world_los_scene.rs`), ballistics_agreement 339, building_interior 104; 562 shared test lines.

## B6. Build specifics

- `cfg(target_arch)` density: editor 598 uses in 151 of 401 files; pages 340 in 91 of 325; core 116 in 34 of 166; debug 19 in 6 of 53.
- 732 `pub(crate)` items must be reviewed.
- Tailwind scans a single source glob, `style/aegis.css:8` (`@source "../src/**/*.rs"`). New crates need added `@source` lines or classes will disappear.
- Trunk builds only the bin.

## B — proposed crates

| Crate | Contents | ~Prod lines | Blocker |
|---|---|---|---|
| `frontend_dto` | core/api/dto | 3.4k | re-exports map-engine scenario/store types |
| `frontend_ui` | core/ui + utils | 2.0k | editor tokens (`slider:19`, `select:15`, `search_box:18`) → move into ui; `gates.rs` → auth crate |
| `frontend_session` | core/api client/sse/endpoints + core/auth (merged, or broken apart with a token-provider trait) | 3.8k | api↔auth cycle; `store.rs:259` editor purge → callback registry; `route_guard.rs:28` → move route tier table into a `frontend_routes_table` crate |
| `frontend_offline` | core/offline | 1.4k | none (depends on session + SW crate) |
| `frontend_map_view` | core/map_view | 0.9k | wasm-only map-engine features |
| `frontend_mission_review` | pages/mission_hub/mission_review | 1.0k | removes administration → mission_hub |
| `frontend_pages_admin` | pages/administration | 15.0k | depends on mission_review; `pins.rs` source pins |
| `frontend_pages_operations` | pages/operations | 5.0k | none |
| `frontend_pages_missions` | pages/mission_hub minus mission_review | 4.7k | review_workspace mounts the editor → keep in the app crate, or depend on the editor crate; `format_bytes` → utils |
| `frontend_pages_doctrine` | pages/doctrine_and_info | 4.7k | none |
| `frontend_pages_field_tools` | pages/field_tools | 4.0k | map_view, offline |
| `frontend_pages_small` | command_center + account | 2.2k | none |
| `frontend_editor` | apps/editor (single crate) | 45k (+37k tests) | internal tangle; `pub(crate)` |
| `frontend_debug` | apps/debug | 6.0k | none |
| `website-frontend` (bin) | main, router, app_routes, navigation | 1.4k | navigation ↔ `app_routes` |

Two things need deciding before any move:
1. **Edition:** align the frontend from 2021 to 2024.
2. **Source-text pin tests:** decide how the `include_str!` pins (`core/test_support/pins.rs`, editor) should work across crate boundaries.
