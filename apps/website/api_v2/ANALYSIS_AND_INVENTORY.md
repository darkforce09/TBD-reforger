# Legacy API Forensic Analysis & Inventory Catalog

> **This is the pre-refactor inventory, kept for reference.** Its census, line counts and proposed
> destinations describe the source tree the refactor started from, not the one that shipped — for
> the file names, route table and boundaries that exist today, read [`README.md`](./README.md) and
> the module `README.md` files under `src/`.

Exhaustive forensic analysis of the pre-refactor backend layout, capturing exact line counts, functional responsibilities, architectural violations, and target refactoring destinations under `apps/website/api_v2/`.

---

## 1. Complete File Census & Line Count Audit

Full inventory of all 78 Rust files in the pre-refactor `src/`, totaling **30,812 lines of code**:

| File Path | Total LOC | Prod LOC | Test LOC | Target `api_v2/` Subsystem |
|:---|:---:|:---:|:---:|:---|
| `handlers/events/events.rs` | **3,022** | 2,676 | 346 | `operations/handlers/` |
| `handlers/missions/missions.rs` | **2,912** | 2,114 | 798 | `missions/handlers/` |
| `handlers/telemetry/telemetry.rs` | **1,804** | 1,234 | 570 | `match_telemetry/handlers/` |
| `app.rs` | **1,641** | 1,243 | 398 | `core/http_router.rs` & `core/observability/` |
| `contract/generated/faction_library.rs` | 1,282 | 1,282 | 0 | `missions/contract/generated/` (codegen) |
| `handlers/admin/admin.rs` | **1,175** | 843 | 332 | `administration/handlers/` & `server_infrastructure/` |
| `handlers/auth/oauth.rs` | **1,100** | 596 | 504 | `identity_and_access/handlers/` |
| `services/mission_compile.rs` | **1,071** | 133 | 938 | `missions/services/` |
| `config.rs` | **995** | 497 | 498 | `core/configuration/` |
| `middleware/ratelimit.rs` | **889** | 483 | 406 | `core/middleware/` |
| `contract/generated/registry_items.rs` | 781 | 781 | 0 | `missions/contract/generated/` (codegen) |
| `handlers/telemetry/servers.rs` | **761** | 658 | 103 | `server_infrastructure/handlers/` |
| `contract/validate.rs` | **759** | 414 | 345 | `missions/contract/` |
| `handlers/content/cms.rs` | **693** | 513 | 180 | `community_content/handlers/` |
| `handlers/auth/me.rs` | **669** | 486 | 183 | `identity_and_access/handlers/` |
| `services/discord.rs` | **646** | 367 | 278 | `identity_and_access/services/` |
| `db.rs` | **551** | 292 | 259 | `core/database/` & `background_workers/` |
| `handlers/missions/registry.rs` | **525** | 431 | 94 | `missions/handlers/prefab_registry.rs` |
| `handlers/content/modpacks.rs` | **507** | 507 | 0 | `community_content/handlers/` |
| `handlers/telemetry/deployments.rs` | 454 | 454 | 0 | `operations/handlers/deployments.rs` |
| `services/registry_import.rs` | 397 | 397 | 0 | `missions/services/registry_import.rs` |
| `handlers/telemetry/field_tools.rs` | 383 | 383 | 0 | `operations/handlers/` & `missions/handlers/` |
| `contract/generated/registry_compat.rs` | 374 | 374 | 0 | `missions/contract/generated/` (codegen) |
| `realtime.rs` | 369 | 189 | 180 | `core/realtime_hub/` |
| `handlers/missions/approvals.rs` | 364 | 320 | 44 | `missions/handlers/approvals_queue.rs` |
| `contract/generated/mission_editor.rs` | 357 | 357 | 0 | `missions/contract/generated/` (codegen) |
| `handlers/admin/audit.rs` | 353 | 262 | 91 | `administration/handlers/audit_logs.rs` |
| `services/game_agent.rs` | 328 | 188 | 140 | `server_infrastructure/services/` |
| `services/audit_notify.rs` | 313 | 284 | 29 | `administration/services/audit_notifier.rs` |
| `services/role_sync.rs` | 311 | 189 | 122 | `identity_and_access/services/` |
| `services/text.rs` | 310 | 175 | 135 | `core/utils/` |
| `services/mortar.rs` | 272 | 135 | 137 | Transition to `website-map-engine` |
| `handlers/auth/auth.rs` | 266 | 241 | 25 | `identity_and_access/handlers/session_tokens.rs` |
| `services/webhook.rs` | 262 | 167 | 95 | `community_content/services/` |
| `handlers/telemetry/leaderboards.rs` | 256 | 194 | 62 | `command_center/handlers/` |
| `handlers/content/wiki.rs` | 250 | 250 | 0 | `community_content/handlers/` |
| `handlers/telemetry/dashboard.rs` | 213 | 213 | 0 | `command_center/handlers/` |
| `models/mission.rs` | 204 | 204 | 0 | `missions/models/mission.rs` |
| `handlers/mod.rs` | 190 | 190 | 0 | Replaced by domain sub-routers |
| `handlers/events/factions.rs` | 180 | 180 | 0 | `missions/handlers/faction_library.rs` |
| `models/event.rs` | 178 | 178 | 0 | `operations/models/event.rs` |
| `contract/generated/loadout.rs` | 177 | 177 | 0 | `missions/contract/generated/` (codegen) |
| `models/telemetry.rs` | 162 | 162 | 0 | `match_telemetry/models/telemetry.rs` |
| `services/user_stats.rs` | 134 | 134 | 0 | `command_center/services/user_stats.rs` |
| `models/content.rs` | 132 | 132 | 0 | `community_content/models/content.rs` |
| `auth/jwt.rs` | 127 | 82 | 45 | `identity_and_access/auth_primitives/` |
| `handlers/auth/dev.rs` | 122 | 122 | 0 | `identity_and_access/handlers/` |
| `middleware/auth.rs` | 122 | 122 | 0 | `core/middleware/authentication.rs` |
| `models/admin.rs` | 118 | 118 | 0 | `FireMission` moved to operations; remainder to admin |
| `models/user.rs` | 114 | 114 | 0 | `administration/models/user.rs` |
| `bin/api.rs` | 113 | 113 | 0 | Updated boot sequence |
| `models/registry.rs` | 97 | 97 | 0 | `missions/models/registry.rs` |
| `state.rs` | 85 | 85 | 0 | `core/application_state.rs` |
| `bin/import_registry.rs` | 83 | 83 | 0 | CLI utility |
| `error.rs` | 79 | 79 | 0 | `core/error_handling/` |
| `models/serde_helpers.rs` | 79 | 79 | 0 | `core/serde_helpers/` |
| `auth/tokens.rs` | 78 | 37 | 41 | `identity_and_access/auth_primitives/` |
| `services/http_retry.rs` | 64 | 64 | 0 | `core/utils/` |
| `middleware/request_id.rs` | 63 | 63 | 0 | `core/middleware/tracing_correlation.rs` |
| `services/ratelimit_gc.rs` | 63 | 63 | 0 | `background_workers/ratelimit_cleanup_worker.rs` |
| `handlers/content/announcements.rs` | 61 | 61 | 0 | `community_content/handlers/` |
| `middleware/cors.rs` | 58 | 58 | 0 | `core/middleware/cross_origin.rs` |
| `services/mod.rs` | 52 | 52 | 0 | Distributed to domain subsystems |
| `middleware/mod.rs` | 48 | 48 | 0 | `core/middleware/` |
| `services/token_purge.rs` | 48 | 48 | 0 | `background_workers/token_purge_worker.rs` |
| `services/audit.rs` | 40 | 40 | 0 | `administration/services/audit_writer.rs` |
| `models/mod.rs` | 33 | 33 | 0 | Distributed to domain subsystems |
| `models/faction.rs` | 28 | 28 | 0 | `missions/models/faction.rs` |
| `lib.rs` | 20 | 20 | 0 | Replaced by `api_v2` library root |
| `handlers/telemetry/mod.rs` | 17 | 17 | 0 | Replaced by domain sub-routers |
| `contract/generated/mod.rs` | 16 | 16 | 0 | `missions/contract/generated/` |
| `contract/mod.rs` | 15 | 15 | 0 | `missions/contract/` |
| `handlers/auth/mod.rs` | 14 | 14 | 0 | Replaced by domain sub-routers |
| `handlers/missions/mod.rs` | 13 | 13 | 0 | Replaced by domain sub-routers |
| `handlers/events/mod.rs` | 12 | 12 | 0 | Replaced by domain sub-routers |
| `handlers/admin/mod.rs` | 12 | 12 | 0 | Replaced by domain sub-routers |
| `handlers/content/mod.rs` | 11 | 11 | 0 | Replaced by domain sub-routers |
| `auth/mod.rs` | 8 | 8 | 0 | Replaced by domain sub-routers |

---

## 2. Forensic Analysis of the 17 Monoliths (>500 LOC)

The "Decomposition Target" names below are the proposal, not the shipped file names — several
modules were split differently once the code was read in full. `src/missions/contract/`, for
instance, is `schema_validators.rs` plus `zone_quantisation.rs` rather than a single
`validate.rs`. The module `README.md` files under `src/` list what each directory actually holds.

### 1. `handlers/events/events.rs` (3,022 LOC)
- **Primary Concerns**: Event CRUD, status derivation, background convergence ticker, Concurrency Gate G7b slot reservations, waitlists, ORBAT slot materialization, squad reservations, leader slot assignments, and server roster synchronization.
- **Decomposition Target (<450 LOC per module under `operations/handlers/`)**:
  - `events_crud.rs` (~420 LOC): Event container CRUD, list filters, and dossier view.
  - `event_lifecycle.rs` (~360 LOC): State transitions and SQL effective status derivation.
  - `registration.rs` (~390 LOC): Gate G7b two-level transactional lock, seat release, waitlists.
  - `orbat_structure.rs` (~430 LOC): Squad reservations, leader slot assignment, roster sync.
  - `deployments.rs` (~450 LOC): Re-homed from telemetry; member service records and LOA.
  - `fire_missions.rs` (~290 LOC): Saved fire missions persistence.
  - Sibling test file: `operations/tests/events.rs` (absorbing 346 LOC of inlined tests).

### 2. `handlers/missions/missions.rs` (2,912 LOC)
- **Primary Concerns**: Scenario catalog, SemVer 2.0 parser, CAD payload validation, version rollback, armory replacements, bookmarks, export download, `/compiled` server compilation, and override metrics.
- **Decomposition Target (<450 LOC per module under `missions/handlers/`)**:
  - `library.rs` (~240 LOC): Filterable scenario catalog, search, and bookmarks.
  - `lifecycle.rs` (~320 LOC): Creation, updates, review submission, and soft-delete.
  - `upload_ingest.rs` (~360 LOC): Version save, SemVer parsing, and rollback.
  - `armory_catalog.rs` (~190 LOC): Virtual arsenal and cargo capacity checks.
  - `compiler.rs` (~340 LOC): Runtime compilation bridge and diagnostic headers.
  - `validation.rs` (~180 LOC): Common string, enum, and authorization predicates.
  - Sibling test file: `missions/tests/missions.rs` (absorbing 798 LOC of inlined tests).

### 3. `handlers/telemetry/telemetry.rs` (1,804 LOC)
- **Primary Concerns**: Dedicated server status heartbeat, history samples, match results ingest, player counter updates, attendance backfill, and AAR replay link validation.
- **Decomposition Target (<350 LOC per module under `match_telemetry/handlers/`)**:
  - `server_heartbeat.rs` (~250 LOC): Ingests ticks, merges via `COALESCE`, logs low-FPS warnings.
  - `match_results.rs` (~320 LOC): Match results, player stat counters, session tokens.
  - `operations/services/participation_attribution.rs`: Finalized exact-match participation and correction provenance, independent of reservation state.
  - `replay_streamer.rs` (~180 LOC): AAR replay URL validation and tick streaming.
  - Sibling test file: `match_telemetry/tests/telemetry.rs` (absorbing 570 LOC of inlined tests).

### 4. `app.rs` (1,641 LOC)
- **Primary Concerns**: Prometheus metrics engine (371 LOC), Postgres durable rate limiter (145 LOC), 98-route monolithic flat router (327 LOC), health probe (183 LOC), and inline test suite (398 LOC).
- **Decomposition Target**:
  - `core/http_router.rs` (<150 LOC): Lean router mounting domain sub-routers.
  - `core/observability/metrics.rs` (~200 LOC): Prometheus registry and latency histograms.
  - `core/observability/health_probe.rs` (~150 LOC): Bounded 200/503 health probe.
  - `core/middleware/durable_ratelimit.rs` (~145 LOC): Postgres token bucket engine.
  - Sibling test file: `core/tests/http_router.rs` (absorbing 398 LOC of inlined tests).

### 5. `handlers/admin/admin.rs` (1,175 LOC)
- **Primary Concerns**: Personnel roster (83 LOC), disciplinary bans/warnings (210 LOC), role updates (100 LOC), RCON console (428 LOC), and inlined tests (332 LOC).
- **Decomposition Target**:
  - `administration/handlers/personnel_roster.rs` (~120 LOC): Member list and warning counts.
  - `administration/handlers/disciplinary.rs` (~200 LOC): Ban, unban, warning issuance, token revocation.
  - `administration/handlers/role_management.rs` (~100 LOC): Role assignments and guild sync.
  - `server_infrastructure/handlers/rcon_console.rs` (~250 LOC): RCON console command execution.
  - Sibling test file: `administration/tests/admin.rs` (absorbing 332 LOC of inlined tests).

### 6. `handlers/auth/oauth.rs` (1,100 LOC)
- **Primary Concerns**: Discord OAuth2 exchange, CSRF cookie validation, host alignment verification, user upsert, role synchronization, error redirects, and inline tests (504 LOC).
- **Decomposition Target**:
  - `identity_and_access/handlers/discord_oauth.rs` (~380 LOC): Login redirect and callback.
  - `identity_and_access/handlers/oauth_host_guard.rs` (~150 LOC): Host alignment and CSRF cookie.
  - Sibling test file: `identity_and_access/tests/oauth.rs` (absorbing 504 LOC of inlined tests).

### 7. `services/mission_compile.rs` (1,071 LOC)
- **Primary Concerns**: Bridge to `website-map-engine` (133 LOC prod) overwhelmed by a 938 LOC inlined test suite.
- **Decomposition Target**:
  - `missions/services/mission_compile.rs` (~140 LOC): Production compiler bridge and diagnostic headers.
  - Sibling test file: `missions/services/tests/mission_compile.rs` (absorbing 938 LOC of inlined tests).

### 8. `config.rs` (995 LOC)
- **Primary Concerns**: 19 environment variables (102 LOC), custom CIDR parser and bitwise IP math (112 LOC), lifecycle validation (241 LOC), and inlined tests (498 LOC).
- **Decomposition Target**:
  - `core/configuration/mod.rs` (~240 LOC): Config struct, loader, and accessors.
  - `core/configuration/proxy_network.rs` (~120 LOC): ProxyNet CIDR parser and bitwise IP matching.
  - Sibling test file: `core/configuration/tests/config.rs` (absorbing 498 LOC of inlined tests).

### 9. `middleware/ratelimit.rs` (889 LOC)
- **Primary Concerns**: L1 in-memory limiter, L2 Postgres limiter, `X-Forwarded-For` trusted proxy scanning, rate-limit seam, and inlined tests (406 LOC).
- **Decomposition Target**:
  - `core/middleware/rate_limiting.rs` (~220 LOC): RateLimitState and middleware layer.
  - `core/middleware/client_identity.rs` (~150 LOC): Forwarded-For inspection and peer resolution.
  - Sibling test file: `core/middleware/tests/ratelimit.rs` (absorbing 406 LOC of inlined tests).

### 10. `handlers/telemetry/servers.rs` (761 LOC)
- **Primary Concerns**: Server CRUD (350 LOC), cached reads (150 LOC), modpack binding (150 LOC), and inlined tests (103 LOC).
- **Decomposition Target**:
  - `server_infrastructure/handlers/server_registry.rs` (~250 LOC): Server CRUD.
  - `server_infrastructure/handlers/health_monitor.rs` (~260 LOC): Cached reads and status stream.
  - `server_infrastructure/handlers/modpack_binding.rs` (~100 LOC): Modpack validation and prefetch.
  - Sibling test file: `server_infrastructure/tests/servers.rs` (absorbing 103 LOC of inlined tests).

### 11. `contract/validate.rs` (759 LOC)
- **Primary Concerns**: JSON Schema draft 2020-12 validation, zone coordinate quantisation, cargo capacity scan (414 LOC prod), and inlined tests (345 LOC).
- **Decomposition Target**:
  - `missions/contract/validate.rs` (~420 LOC): Production validator and quantisation.
  - Sibling test file: `missions/contract/tests/validate.rs` (absorbing 345 LOC of inlined tests).

### 12. `handlers/content/cms.rs` (693 LOC)
- **Primary Concerns**: Announcement CRUD, Discord webhook dispatch, image multipart uploads (513 LOC prod), and inlined tests (180 LOC).
- **Decomposition Target**:
  - `community_content/handlers/announcements_admin.rs` (~350 LOC): CMS announcement CRUD.
  - `community_content/handlers/discord_webhook_push.rs` (~150 LOC): Webhook dispatch.
  - `community_content/handlers/media_upload.rs` (~180 LOC): Image multipart uploads.
  - Sibling test file: `community_content/tests/cms.rs` (absorbing 180 LOC of inlined tests).

### 13. `handlers/auth/me.rs` (669 LOC)
- **Primary Concerns**: User profile echo, Arma 6-digit link code generation, status polling, unlinking, and game link confirm (486 LOC prod) with inlined tests (183 LOC).
- **Decomposition Target**:
  - `identity_and_access/handlers/member_profile.rs` (~100 LOC): Profile retrieval and update.
  - `identity_and_access/handlers/arma_linking.rs` (~380 LOC): Link code generation, confirm, unlink.
  - Sibling test file: `identity_and_access/tests/me.rs` (absorbing 183 LOC of inlined tests).

### 14. `services/discord.rs` (646 LOC)
- **Primary Concerns**: Discord OAuth2 token exchange, user profile, guild member lookup, CDN URL validation (367 LOC prod), and inlined tests (278 LOC).
- **Decomposition Target**:
  - `identity_and_access/services/discord_client.rs` (~370 LOC): Discord API client.
  - Sibling test file: `identity_and_access/services/tests/discord.rs` (absorbing 278 LOC of inlined tests).

### 15. `db.rs` (551 LOC)
- **Primary Concerns**: Connection pool creation, tuning from env, backoff retry, migration runner, and inlined leaderboard MV refresh ticker (292 LOC prod) with inlined tests (259 LOC).
- **Decomposition Target**:
  - `core/database/connection_pool.rs` (~120 LOC): Pool creation and tuning.
  - `core/database/mod.rs` (~180 LOC): Connection retry and migration runner.
  - `background_workers/leaderboard_refresher.rs` (~70 LOC): MV refresh extracted to worker.
  - Sibling test file: `core/database/tests/db.rs` (absorbing 259 LOC of inlined tests).

### 16. `handlers/missions/registry.rs` (525 LOC)
- **Primary Concerns**: Item catalog and compatibility graph with weak ETag caching (431 LOC prod) and inlined tests (94 LOC).
- **Decomposition Target**:
  - `missions/handlers/prefab_registry.rs` (~430 LOC): Item registry and compatibility graph.
  - Sibling test file: `missions/tests/prefab_registry.rs` (absorbing 94 LOC of inlined tests).

### 17. `handlers/content/modpacks.rs` (507 LOC)
- **Primary Concerns**: Modpack listing, current modpack lookup, authoring, nested mod replace, and active toggle (507 LOC prod).
- **Decomposition Target**:
  - `community_content/handlers/modpack_catalog.rs` (~160 LOC): Public list and current modpack queries.
  - `community_content/handlers/modpack_admin.rs` (~340 LOC): Modpack authoring, replace, and active toggle.

---

## 3. Complete HTTP Route Inventory (108 Routes)

| Path | Method | Legacy Handler | Target Domain | Auth Tier |
|:---|:---|:---|:---|:---|
| `/healthz` | GET | `app::healthz` | `core/observability` | Public / `X-Service-Token` |
| `/metrics` | GET | `app::metrics_scrape` | `core/observability` | `ServiceAuth` |
| `/uploads/*` | GET | `ServeDir::new("uploads")` | `core` (static) | Public |
| `/map-assets/*` | GET | `ServeDir::new(map_assets)` | `core` (static, exempt) | Public |
| `/*` (fallback) | GET | SPA fallback | `core` (fallback) | Public |
| `/api/v1/auth/discord/login` | GET | `handlers::oauth::discord_login` | `identity_and_access` | Public |
| `/api/v1/auth/discord/callback` | GET | `handlers::oauth::discord_callback` | `identity_and_access` | Public |
| `/api/v1/auth/refresh` | POST | `handlers::auth::refresh` | `identity_and_access` | Public |
| `/api/v1/auth/logout` | POST | `handlers::auth::logout` | `identity_and_access` | Public |
| `/api/v1/auth/dev-login` | GET | `handlers::dev::dev_login` | `identity_and_access` | Dev mode only |
| `/api/v1/me` | GET, PATCH | `handlers::me::get_me`, `update_me` | `identity_and_access` | `AuthUser` |
| `/api/v1/me/link` | POST, DELETE | `handlers::me::create_link_code`, `unlink` | `identity_and_access` | `AuthUser` |
| `/api/v1/me/link/status` | GET | `handlers::me::link_status` | `identity_and_access` | `AuthUser` |
| `/api/v1/me/deployments` | GET | `handlers::deployments::get_my_deployments` | `operations` | `AuthUser` |
| `/api/v1/me/leave-requests` | GET, POST | `handlers::deployments::list_my_leave`, `submit_leave` | `operations` | `AuthUser` |
| `/api/v1/ingest/link-confirm` | POST | `handlers::me::ingest_link_confirm` | `identity_and_access` | `ServiceAuth` |
| `/api/v1/ingest/server-status` | POST | `handlers::telemetry::ingest_server_status` | `match_telemetry` | `ServiceAuth` |
| `/api/v1/ingest/match-results` | POST | `handlers::telemetry::ingest_match_results` | `match_telemetry` | `ServiceAuth` |
| `/api/v1/ingest/missions` | GET | `handlers::missions::ingest_list_missions` | `missions` | `ServiceAuth` |
| `/api/v1/ingest/events/{id}/roster` | GET | `handlers::events::ingest_event_roster` | `operations` | `ServiceAuth` |
| `/api/v1/announcements` | GET | `handlers::announcements::list_announcements` | `community_content` | `AuthUser` |
| `/api/v1/announcements/{id}` | GET | `handlers::announcements::get_announcement` | `community_content` | `AuthUser` |
| `/api/v1/wiki` | GET | `handlers::wiki::list_wiki` | `community_content` | `AuthUser` |
| `/api/v1/wiki/{slug}` | GET, PUT | `handlers::wiki::get_wiki_page`, `upsert_wiki_page` | `community_content` | GET: `AuthUser`, PUT: `AdminUser` |
| `/api/v1/vehicle-database` | GET, POST | `handlers::wiki::list_vehicles`, `create_vehicle` | `community_content` | GET: `AuthUser`, POST: `AdminUser` |
| `/api/v1/modpacks` | GET, POST | `handlers::modpacks::list_modpacks`, `create_modpack` | `community_content` | GET: `AuthUser`, POST: `AdminUser` |
| `/api/v1/modpacks/current` | GET | `handlers::modpacks::get_current_modpack` | `community_content` | `AuthUser` |
| `/api/v1/modpacks/{id}` | PUT, DELETE | `handlers::modpacks::replace_modpack`, `delete_modpack` | `community_content` | `AdminUser` |
| `/api/v1/modpacks/{id}/set-current` | POST | `handlers::modpacks::set_current_modpack` | `community_content` | `AdminUser` |
| `/api/v1/servers` | GET, POST | `handlers::servers::list_servers`, `create_server` | `server_infrastructure` | GET: `AuthUser`, POST: `AdminUser` |
| `/api/v1/servers/{id}` | PATCH, DELETE | `handlers::servers::update_server`, `deactivate_server` | `server_infrastructure` | `AdminUser` |
| `/api/v1/servers/{id}/status` | GET | `handlers::servers::get_server_status` | `server_infrastructure` | `AuthUser` |
| `/api/v1/servers/{id}/status/stream` | GET | `handlers::leaderboards::stream_server_status` | `server_infrastructure` | `AuthUser` (SSE) |
| `/api/v1/registry` | GET | `handlers::registry::list_registry` | `missions` | `AuthUser` |
| `/api/v1/registry/compat` | GET | `handlers::registry::list_registry_compat` | `missions` | `AuthUser` |
| `/api/v1/factions` | GET, POST | `handlers::factions::list_factions`, `create_faction` | `missions` | GET: `AuthUser`, POST: `MissionMakerUser` |
| `/api/v1/factions/{id}` | GET, PUT, DELETE | `handlers::factions::get_faction`, `update_faction`, `delete_faction` | `missions` | GET: `AuthUser`, PUT/DEL: `MissionMakerUser` |
| `/api/v1/dashboard` | GET | `handlers::dashboard::get_dashboard` | `command_center` | `AuthUser` |
| `/api/v1/leaderboards` | GET | `handlers::leaderboards::get_leaderboards` | `command_center` | `AuthUser` |
| `/api/v1/users/{discordId}/stats` | GET | `handlers::leaderboards::get_user_stats` | `command_center` | `AuthUser` |
| `/api/v1/admin/leave-requests` | GET | `handlers::deployments::list_all_leave` | `operations` | `AdminUser` |
| `/api/v1/admin/leave-requests/{id}` | PATCH | `handlers::deployments::review_leave` | `operations` | `AdminUser` |
| `/api/v1/admin/audit-logs` | GET | `handlers::audit::list_audit_logs` | `administration` | `AdminUser` |
| `/api/v1/admin/audit-logs/stream` | GET | `handlers::audit::stream_audit_logs` | `administration` | `AdminUser` (SSE) |
| `/api/v1/admin/audit-logs/export.csv` | GET | `handlers::audit::export_audit_logs_csv` | `administration` | `AdminUser` |
| `/api/v1/admin/users` | GET | `handlers::admin::list_users` | `administration` | `AdminUser` |
| `/api/v1/admin/users/{discordId}` | PATCH | `handlers::admin::update_user` | `administration` | `AdminUser` |
| `/api/v1/admin/users/{discordId}/ban` | POST, DELETE | `handlers::admin::ban_user`, `unban_user` | `administration` | `AdminUser` |
| `/api/v1/admin/users/{discordId}/warnings` | POST | `handlers::admin::issue_warning` | `administration` | `AdminUser` |
| `/api/v1/admin/roles/sync` | POST | `handlers::admin::resync_roles` | `administration` | `AdminUser` |
| `/api/v1/admin/servers/{id}/rcon` | POST | `handlers::admin::send_rcon` | `server_infrastructure` | `AdminUser` |
| `/api/v1/admin/mission-default-overrides` | GET | `handlers::missions::mission_default_overrides` | `missions` | `AdminUser` |
| `/api/v1/missions` | GET, POST | `handlers::missions::list_missions`, `create_mission` | `missions` | GET: `AuthUser`, POST: `MissionMakerUser` |
| `/api/v1/missions/{id}` | GET, PATCH, DELETE | `handlers::missions::get_mission`, `update_mission`, `delete_mission` | `missions` | GET: `AuthUser`, PATCH/DEL: `MissionMakerUser` |
| `/api/v1/missions/{id}/submit` | POST | `handlers::missions::submit_mission` | `missions` | `MissionMakerUser` |
| `/api/v1/missions/{id}/versions` | POST | `handlers::missions::create_version` | `missions` | `MissionMakerUser` (256 MB cap) |
| `/api/v1/missions/{id}/versions/{vid}` | GET | `handlers::missions::get_version` | `missions` | `AuthUser` |
| `/api/v1/missions/{id}/versions/{vid}/set-current` | POST | `handlers::missions::set_current_version` | `missions` | `MissionMakerUser` |
| `/api/v1/missions/{id}/armory` | GET, PUT | `handlers::missions::get_armory`, `set_armory` | `missions` | GET: `AuthUser`, PUT: `MissionMakerUser` |
| `/api/v1/missions/{id}/bookmark` | POST, DELETE | `handlers::missions::bookmark_mission`, `remove_bookmark` | `missions` | `AuthUser` |
| `/api/v1/missions/{id}/export` | GET | `handlers::missions::export_mission` | `missions` | `AuthUser` |
| `/api/v1/missions/{id}/compiled` | GET | `handlers::missions::get_compiled_mission` | `missions` | `AuthUser` |
| `/api/v1/events` | GET, POST | `handlers::events::list_events`, `create_event` | `operations` | GET: `AuthUser`, POST: `AdminUser` |
| `/api/v1/events/{id}` | GET, PATCH, DELETE | `handlers::events::get_event`, `update_event`, `delete_event` | `operations` | GET: `AuthUser`, PATCH/DEL: `AdminUser` |
| `/api/v1/events/{id}/missions` | POST | `handlers::events::add_event_mission` | `operations` | `AdminUser` |
| `/api/v1/events/{id}/missions/{emid}` | DELETE | `handlers::events::remove_event_mission` | `operations` | `AdminUser` |
| `/api/v1/event-missions/{emid}/orbat` | GET | `handlers::events::get_orbat` | `operations` | `AuthUser` |
| `/api/v1/event-missions/{emid}/register` | POST, DELETE | `handlers::events::register_for_event_mission`, `withdraw_from_event_mission` | `operations` | `AuthUser` |
| `/api/v1/event-missions/{emid}/slots/{slotId}/assign` | PUT, DELETE | `handlers::events::assign_slot`, `clear_slot` | `operations` | `LeaderUser` |
| `/api/v1/event-missions/{emid}/squads/reserve` | POST | `handlers::events::reserve_squad` | `operations` | `LeaderUser` |
| `/api/v1/event-missions/{emid}/squads/release` | POST | `handlers::events::release_squad` | `operations` | `LeaderUser` |
| `/api/v1/members` | GET | `handlers::events::search_members` | `operations` | `AuthUser` |
| `/api/v1/approvals` | GET | `handlers::approvals::list_approvals` | `missions` | `AdminUser` |
| `/api/v1/approvals/{id}/approve` | POST | `handlers::approvals::approve_mission` | `missions` | `AdminUser` |
| `/api/v1/approvals/{id}/reject` | POST | `handlers::approvals::reject_mission` | `missions` | `AdminUser` |
| `/api/v1/fire-missions/solve` | POST | `handlers::field_tools::solve_fire` | `operations` | `AuthUser` |
| `/api/v1/fire-missions` | POST | `handlers::field_tools::save_fire` | `operations` | `AuthUser` |
| `/api/v1/events/{id}/fire-missions` | GET | `handlers::field_tools::list_event_fire_missions` | `operations` | `AuthUser` |
| `/api/v1/missions/{id}/inject` | POST | `handlers::field_tools::inject_mission` | `missions` | `MissionMakerUser` |
| `/api/v1/cms/announcements` | GET, POST | `handlers::cms::list_cms_announcements`, `create_announcement` | `community_content` | `AdminUser` |
| `/api/v1/cms/announcements/{id}` | PATCH, DELETE | `handlers::cms::update_announcement`, `delete_announcement` | `community_content` | `AdminUser` |
| `/api/v1/cms/announcements/{id}/push-discord` | POST | `handlers::cms::push_announcement_discord` | `community_content` | `AdminUser` |
| `/api/v1/cms/uploads` | POST | `handlers::cms::upload_image` | `community_content` | `AdminUser` (6 MB cap) |

---

## 4. Monorepo Law Violations Cataloged

### Law 7 Violations (File Length & Inline Tests)
- **17 production files** exceed 500 lines of code.
- **Massive inline test modules**:
  - `services/mission_compile.rs`: 938 lines of inline tests (prod is only 133 lines).
  - `handlers/missions/missions.rs`: 798 lines of inline tests (prod is 2,114 lines).
  - `handlers/telemetry/telemetry.rs`: 570 lines of inline tests (prod is 1,234 lines).
  - `handlers/auth/oauth.rs`: 504 lines of inline tests (prod is 596 lines).
  - `config.rs`: 498 lines of inline tests (prod is 497 lines).
  - `middleware/ratelimit.rs`: 406 lines of inline tests (prod is 483 lines).
  - `app.rs`: 398 lines of inline tests (prod is 1,243 lines).
  - `handlers/events/events.rs`: 346 lines of inline tests (prod is 2,676 lines).
  - `contract/validate.rs`: 345 lines of inline tests (prod is 414 lines).
  - `handlers/admin/admin.rs`: 332 lines of inline tests (prod is 843 lines).
  - `services/discord.rs`: 278 lines of inline tests (prod is 367 lines).
  - `db.rs`: 259 lines of inline tests (prod is 292 lines).

### Law 8 Violations (Historical Transition Commentary)
- `Cargo.toml:6`: the crate description named the implementation the code had been ported from, with a work-item id.
- `lib.rs:1-6`: the crate header described the port, the plan file it followed, and the package layout it mirrored.
- `missions.rs` (thirteen sites): comments cited the port and eleven tickets instead of the behaviour.
- `approvals.rs:1, 263-320`: 58 lines of commentary arguing one ticket's decision.
- `mortar.rs:1, 20-33, 68-91`: discussion of earlier bugs and porting details.
- `game_agent.rs:1-27`: a transition narrative across four tickets.
