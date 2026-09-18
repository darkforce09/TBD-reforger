# API V2 Phase Three Handoff

Status: four domains are live on `main` — `identity_and_access` (`b4e6fa075`), `administration` (`0bedc1035`), `community_content` (`b327f7f02`), `server_infrastructure` (`1fb7d759d`), plus the gate fix-up `73893b269`. Their handlers, services, and models left the legacy `src/{handlers,services,models}` directories; every consumer imports the domain paths. Missions, operations, match telemetry, and command center still sit in the legacy directories; Phases Four and Five move them.

## Ownership and layout

Every domain follows the same shape: `mod.rs` (declares `handlers`, `models`, `routes`, `services`; re-exports `routes`), `routes.rs` (the `/api/v1`-relative table, handlers referenced as `handlers::<file>::<fn>`), `handlers/`, `services/`, `models/`, each with a sibling `tests/` directory declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

- `identity_and_access/` (27 files): Discord OAuth (`discord_oauth.rs`, `oauth_host_guard.rs`), session tokens, member profile, Arma link codes and confirmation, developer login; services for session issuance, user lookup (`load_user`), the Discord client and user profile, Discord role sync, refresh-token purge; `models/user_account.rs` holds `User`, `UserRole`, `DiscordRole`, `UserDiscordRole`, `IdentityLinkCode`, `RefreshToken`.
- `administration/` (16 files): personnel roster, role management, disciplinary actions, audit logs; `services/audit_writer.rs` holds `write_audit` and `actor_display_name` (the former `handlers::username`, same body, 28 call sites); `services/audit_notifier.rs`; `models/{audit_log,warning}.rs`.
- `community_content/` (21 files): public and admin announcements, the Discord push, media upload (owns `UPLOAD_DIR`), wiki knowledgebase, vehicle database, modpack catalog and admin; `services/modpack_lookup.rs` (`ModpackDto`, `load_modpack`, `load_current_modpack`, consumed by server_infrastructure and the dashboard) and `services/discord_webhook.rs` (`WebhookService`); `models/{announcement,wiki,modpack}.rs`.
- `server_infrastructure/` (18 files): server intel and registry, the server-status SSE stream, the RCON command parser and console, `services/game_agent.rs`, `services/status_broadcast.rs` (`publish_server_status`, `publish_all_server_statuses`, previously parked under `core/realtime_hub`), `models/server.rs`.
- `core/application_state.rs` imports `DiscordService` and `WebhookService` from their domains; no other `core/` file imports a domain.
- Legacy remainder: `handlers/{events,missions,telemetry}`, `handlers/mod.rs` (`load_mission` only), `services/{mission_compile,mortar,registry_import,user_stats}.rs`, `models/{admin (FireMission only),event,faction,mission,registry,telemetry (match types only)}.rs`, `contract/`.

## Pins and gates

- Frontend `pages/administration/content_manager/tests/content.rs` asserts the `/cms/announcements` GET and POST registrations as two substrings (`get(handlers::announcements_admin::list_cms_announcements)` and `.post(handlers::announcements_admin::create_announcement)`) because rustfmt wraps the registration across lines.
- Frontend `pages/administration/personnel/tests/personnel.rs` matches the quoted `/admin/roles/sync` path, because rustfmt wraps that registration across lines (its ban/warnings sibling already did the same). The administration table imports `super::handlers` like the other domain tables.
- Source-text self-pins moved with their tests and now read the split file each windows (`include_str!("../<file>.rs")` from a `tests/` sibling). The game-agent test includes the tools_v2 agent template with seven `..`.
- `tests/t336_user_stats_service.rs` reads `arma_link_confirmation.rs` and `arma_link_codes.rs` in place of the former `me.rs`.
- Allowlist: seven `SIZE-3` exemptions retired (`handlers/auth/{me,oauth}.rs`, `services/discord.rs`, `handlers/content/{cms,modpacks}.rs`, `handlers/admin/admin.rs`, `handlers/telemetry/servers.rs`). Twelve `api_v2` entries remain (six production, six test suites), all owned by Phases Four to Six.

## Verification evidence

Logs under the session scratchpad `phase3/` (`3_1_*` to `3_5_*`).

- Every sub-phase: `cargo check -p website-api --all-targets` (0 warnings), `cargo test -p website-api --lib --bins` (282 passed, 0 failed), `cargo fmt --all --check`, `cargo xtask verify file-length` (0 violations), `cargo xtask verify route-tags` (PASS, 103 tags against 103 routes in 8 route files) — all green, re-run by the orchestrator before each commit.
- Phase gate: `cargo check --workspace --locked` pass (245 warnings, all in `website-frontend`, identical to the Phase 1 baseline); `cargo xtask ticket check` OK; `cargo xtask verify file-length` 2424 files, 0 violations.
- `cargo test -p xtask -p ticket-engine`: 845 passed, 0 failed.
- `cargo xtask db test-it`: 44 targets, 785 passed, 0 failed, 1 ignored (identical to Phase 2).
- `cargo xtask mk ci-local-leptos`: pass (rc 0; 1342 frontend tests, trunk release build) in the private target dir.
- `cargo xtask ci ci-local`: pass (rc 0), log `phase3/3_5_ci-local.log`.

## Working-tree handling

- The other session's ten tracked-but-deleted paths are staged only for the duration of `ci ci-local` and restored with `git reset -- <paths>` afterwards; no Phase 3 commit contains them.
- `cargo xtask mk …` recipes run the target-directory ABI guard; the shared `target-container/` is stamped by another session's glibc 2.43 container, so the frontend gate ran with `CARGO_TARGET_DIR=…/target-container-api-v2` (private, built from scratch). Everything else used `target-container/` as before.
- The local development database still carries the migration `0021` checksum drift noted in the Phase 2 handoff; integration suites use fresh databases.

## Temporary placements (tracked)

- `core/database/leaderboard_refresh.rs` → `command_center/services/leaderboard_view.rs` in Phase 5.
- `background_workers/event_lifecycle_sweeper.rs` calls `handlers::events::sweep_once` until Phase 4 moves it to `operations/services/event_lifecycle_sweep.rs`.
- `handlers/mod.rs` still hosts `load_mission` until Phase 4.
- `handlers/telemetry/leaderboards.rs` keeps the leaderboard and user-stats handlers until Phase 5.
- Scaffold READMEs under the domain directories (including the empty `<domain>/tests/` and `identity_and_access/auth_primitives/` placeholders) still describe the blueprint's planned filenames; Phase 6 rewrites or removes them.
