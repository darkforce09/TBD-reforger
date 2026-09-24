# API V2 Phase Four Handoff

Status: the `missions` and `operations` domains are live on `main`, and the mortar ballistics solver lives in `website-map-engine`. Commits: `07fdc868b` (4.1 mortar to map-engine and 4.2 missions contract, validation, models), `334c07e7e` (4.3), `4b809b55f` (4.4), `de321e10e` (4.5), `cb444b9bd` (4.6), `ca4dd5e4a` (4.7). Match telemetry and command center remain under the legacy `handlers/telemetry`, `services/user_stats.rs`, and `models/telemetry.rs`; Phase Five moves them and retires the legacy directories.

## Ownership and layout

- `website-map-engine/src/data/scenario/ballistics/` owns the mortar charge tables and `solve_fire_mission`; the API's fire-mission handler imports it. Engine-layer rule 4 holds (nothing outside `data/scenario` is imported).
- `missions/` (contract, validation, handlers, services, models):
  - `contract/` holds `schema_validators.rs` (the five bundled JSON schemas and the `validate_*` entry points), `zone_quantisation.rs` (the zone projection and radius sharpening that mirrors the compiler's flatten step), `loadout_projection.rs` (hand-maintained), and `generated/` (typify output; the five xtask pins now point here, and `verify-codegen-fresh` passes).
  - `validation/` holds the mission field, semver, version payload, and access predicates as `pub(crate)` functions consumed by the handlers.
  - `handlers/` holds the mission library, lifecycle, versions, armory, export (with the compiled-document route), default overrides, game-server injection (with the ingest listing and staging directory), approvals queue, registry items and compat graph, and the faction library.
  - `services/` holds `mission_lookup` (`load_mission`, `load_mission_or_404`, `mission_title_terrain`, consumed by operations and the dashboard), `mission_document` (`build_mission_doc`), `mission_compile`, and `registry_import` (the `import_registry` binary imports it).
- `operations/` (handlers, services, models):
  - `services/` holds the event status rules (effective-status SQL, transition table), event lookup, the lifecycle sweep the background worker calls, and the ORBAT template re-exports from the map engine.
  - `handlers/` holds event create/update/delete, listing, mission attachment, ORBAT view and member search, slot registration and assignment, roster ingest, member service record, leave requests, and fire missions.
  - `models/` holds the event, leave request, and fire mission rows.
- Legacy remainder: `handlers/telemetry/{telemetry,dashboard,leaderboards}.rs`, `services/user_stats.rs`, `models/telemetry.rs` (match types only), `core/database/leaderboard_refresh.rs`.

## Pins and gates

- Tooling pins for the codegen output: `tools_v2/xtask/src/commands/generate/schema_types.rs`, `commands/ci/{task_definitions,editor_api}.rs`, `verifications/language_bans/node_and_file_limits/repository_access.rs`, `src/tests/node_free_tests.rs`. The mission-editor schema description names `missions/contract/zone_quantisation.rs`.
- The generated `mission_editor.rs` was refreshed; its drift against the committed file predates this phase and `verify-codegen-fresh` was already red at the previous commit.
- Source-text self-pins moved with their tests and window the split file each asserts on; tests that window two files carry two `include_str!` consts. The mission compile tests split into `mission_compile_flatten.rs` and `mission_compile_diagnostics.rs` (two `#[path]` test modules on one production file).
- Frontend comment `apps/editor/shell/document_commands/imp/compilation.rs` names the new compiled-document handler path.
- Allowlist: six production exemptions retired (`contract/validate.rs`, `handlers/missions/{missions,registry}.rs`, `services/mission_compile.rs`, `handlers/events/events.rs`, plus none added). Remaining `api_v2` entries: `handlers/telemetry/telemetry.rs` and the six test suites.

## Deliberate byte changes

- The placeholder modpack name written by `registry_import::ensure_modpack` is `Imported registry export` (it carried a ticket id inside SQL data; nothing reads the string).
- `handlers::username` was renamed `actor_display_name` in Phase 3; no further renames here.

## Verification evidence

Logs under the session scratchpad `phase4/` (`4_1_*` to `4_8_*`).

- Every sub-phase: `cargo check -p website-api --all-targets` (0 warnings), `cargo test -p website-api --lib --bins` (274 passed after the eight mortar tests moved to the map engine, 0 failed), `cargo fmt --all --check`, `cargo xtask verify file-length` (0 violations), `cargo xtask verify route-tags` (PASS, 103 tags against 103 routes) — all green, re-run by the orchestrator before each commit.
- 4.1: `cargo test -p website-map-engine` runs the eight ballistics tests (the pre-existing `feature_gate_tripwire` failure under default features is unrelated); `cargo xtask verify engine-layers` PASS.
- 4.2: `cargo xtask ci schema-codegen` then `cargo xtask ci verify-codegen-fresh` pass; `cargo test -p xtask` 631 passed.
- Phase gate: `cargo fmt --all --check` pass; `cargo check --workspace --locked` pass (245 warnings, all `website-frontend`, the Phase 1 baseline); `cargo clippy -p website-api -p website-map-engine --all-targets --all-features -- -D warnings` clean; `cargo xtask verify route-tags` PASS (103/103, 8 route files); `cargo xtask verify file-length` 2475 files, 0 violations; `cargo xtask ticket check` OK; `cargo test -p xtask -p ticket-engine` 845 passed; `cargo test -p website-map-engine --all-features` 1444 passed, 0 failed; `cargo xtask db test-it` 44 targets, 777 passed, 0 failed, 1 ignored (Phase 3 count minus the eight mortar tests); `cargo xtask mk ci-local-leptos` pass (1342 frontend tests, trunk release build; private target dir); `cargo xtask ci ci-local`: pass (rc 0), log `phase4/4_8_ci-local.log`.
- Gate fix-ups (commit `ae3b96156`): an over-indented doc list in `operations/services/event_status_rules.rs` (clippy `doc_overindented_list_items`), and the mission-editor schema's `zones` description, which rustdoc compiled as a doctest once it flowed into the regenerated projection; the description is now a single present-tense paragraph and the projection regenerated. `verify-codegen-fresh` is a `git diff` of the generated directory, so it reads clean only once that regeneration is committed.

## Working-tree handling

- The other session's ten tracked-but-deleted paths are staged only for the duration of `ci ci-local` and restored afterwards; no Phase 4 commit contains them. Its two `tools_v2/xtask/**/README.md` edits are left unstaged.
- `cargo xtask mk …` recipes use `CARGO_TARGET_DIR=…/target-container-api-v2` (see the Phase 3 handoff).

## Temporary placements and follow-ups (tracked)

- `core/database/leaderboard_refresh.rs` → `command_center/services/leaderboard_view.rs` in Phase 5.
- `services/user_stats.rs` and `services/mod.rs` remain until Phase 5 (`command_center/services/user_stats.rs`).
- `operations/handlers/roster_ingest.rs::load_cargo_phys_catalog` duplicates `missions/handlers/mission_versions.rs::load_cargo_phys_catalog`; a service-layer home in the missions domain would let operations import it. Candidate for Phase 5.3 or Phase 6.
- `operations/handlers/slot_registration.rs` is 488 lines; the Phase 6 Law 8 scrub reduces it.
- Scaffold READMEs under `src/missions/**` and `src/operations/**` still describe the blueprint's planned filenames (some name files that do not exist); Phase 6 rewrites or removes them.
