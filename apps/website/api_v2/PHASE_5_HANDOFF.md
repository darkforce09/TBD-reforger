# API V2 Phase Five Handoff

Status: all eight domains are live on `main` and the legacy `handlers/`, `services/`, `models/`, and `contract/` directories no longer exist. Commits: `bec09956e` (5.1 match_telemetry), `1e98eb1fe` (5.2 command_center and legacy retirement), `5d67a1fa7` (5.3 architecture rules and cargo-catalog dedup). `src/` is now `lib.rs`, `bin/`, `core/`, `background_workers/`, the eight domains, and `tests/architecture_rules.rs`.

## Ownership and layout

- `match_telemetry/`: `handlers/{ingest_parsing, server_heartbeat, match_results_contract, match_results, attendance_attribution, match_upsert}.rs` (the attendance retraction and marking run as two functions on the open ingest transaction, in the original statement order), `models/match_record.rs` (`MissionOutcome`, `Match`, `MatchPlayerStat`).
- `command_center/`: `handlers/{live_dashboard, leaderboards, user_stats_card}.rs`, `services/{user_stats, leaderboard_view}.rs` (the leaderboard materialized-view refresh the background worker and the best-effort recompute call).
- `missions/services/cargo_catalog.rs` owns `load_cargo_phys_catalog`; the mission versions handler and the operations roster ingest import it.
- `src/tests/architecture_rules.rs` (declared from `lib.rs` under `#[cfg(test)]`) walks `src/` (skipping the codegen directory) and fails on: inline test modules, `T-NNN` references, Go-port narrative, `core/` importing a domain outside `application_state.rs` and `http_router.rs`, a domain's handlers/services/models importing another domain's handlers, `background_workers` used outside the binary, a domain without a merged route table, and any legacy top-level module path.

## Pins and gates

- `tests/t336_user_stats_service.rs` reads `command_center/services/user_stats.rs`, `match_telemetry/handlers/{match_results, attendance_attribution}.rs`, `identity_and_access/handlers/{arma_link_confirmation, arma_link_codes}.rs`, and `operations/handlers/member_service_record.rs`.
- `tests/leaderboards_paging.rs` imports `command_center::handlers::leaderboards`; `tests/deployments_combat.rs` imports `command_center::services::leaderboard_view`.
- Allowlist: the last production exemption (`handlers/telemetry/telemetry.rs`) retired; the six `api_v2` test-suite entries remain for Phase 6.

## Verification evidence

Logs under the session scratchpad `phase5/` (`5_1_*` to `5_4_*`).

- Every sub-phase: `cargo check -p website-api --all-targets` (0 warnings), `cargo test -p website-api --lib --bins` (274 passed through 5.2; 282 passed after the architecture tests), `cargo fmt --all --check`, `cargo xtask verify file-length` (0 violations), `cargo xtask verify route-tags` (PASS, 103/103), `cargo clippy -p website-api --all-targets -- -D warnings` — all green, re-run by the orchestrator before each commit.
- Phase gate: `cargo fmt --all --check` pass; `cargo check --workspace --locked` pass (245 warnings, all `website-frontend`, the Phase 1 baseline); `cargo clippy -p website-api -p website-map-engine --all-targets --all-features -- -D warnings` clean; `cargo xtask verify route-tags` PASS (103/103, 8 route files); `cargo xtask verify file-length` 2489 files, 0 violations; `cargo xtask ticket check` OK; `cargo test -p xtask -p ticket-engine` 845 passed; `cargo test -p website-map-engine --all-features` 1444 passed, 0 failed; `cargo test -p website-api --doc` 0 failed (1 ignored); `cargo xtask mk ci-local-leptos` pass (1342 frontend tests, trunk release build; private target dir); `cargo xtask ci ci-local` pass (rc 0, including its `rust-test-it` step; log `phase5/5_4_ci-local.log`); `cargo xtask db test-it` 44 targets, 785 passed, 0 failed, 1 ignored (Phase 4 count plus the eight architecture tests).
- Environment notes from this gate: the Postgres container was stopped and the scratchpad `podman` shim was missing after the session resumed, so the first `db test-it` and `ci ci-local` attempts failed before running any test (`container state improper`, `podman: not found`); both were rerun after `podman start tbd_reforger_db` and recreating the shim. One `db test-it` run that overlapped a concurrent `ci ci-local` build in the same target directory ended with a rustdoc internal compiler error (`no resolution for an import`) during the lib doctest step; the doctests pass in isolation and the final run below was executed alone.

## Working-tree handling

- The other session's ten tracked-but-deleted paths are staged only for the duration of `ci ci-local` and restored afterwards; its `tools_v2/**` edits are left unstaged.
- `cargo xtask mk …` recipes use `CARGO_TARGET_DIR=…/target-container-api-v2`.

## Follow-ups for Phase 6

- Integration suites over 1000 lines: `tests/{telemetry, events, missions, misc_integration, null_tolerance}.rs` and `tests/common/mod.rs` (6.1 to 6.3); eleven ticket-prefixed suite files (6.4); 742 Law 8 hits under `tests/` (6.6).
- `packages/tbd-schema/schema/mission-editor-payload.schema.json` still carries ticket ids in two other descriptions; they flow into `missions/contract/generated/mission_editor.rs` (exempt from the architecture rules). Rewrite them and regenerate in 6.5.
- Scaffold READMEs under every domain (including the empty `<domain>/tests/` and `identity_and_access/auth_primitives/` placeholders) describe the blueprint, not the tree; 6.7 rewrites or removes them.
