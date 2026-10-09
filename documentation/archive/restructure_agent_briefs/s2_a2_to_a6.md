**Status:** archived

# S2 agents A2 to A6: deploy, wave execution, runtime paths, births and documentation

Launch bodies for the second wave of stage S2. Each fills `{BODY}` of the
[R2–R4 template](/documentation/archive/restructure_agent_briefs/s1_r2_to_r4_template.md) with these
changes to its fixed text: stage S2; "A1 has just moved the website folder's crates to `apps/` and
`legacy/`, renamed every package to its snake_case folder name (the fleet host agent everywhere,
D17) and created the root `deploy/` folder with the relocation tool (manifest
`documentation/restructure/manifests/s2_apps_and_deploy.tsv`); the tree compiles"; A1's report is
`<scratch>/report_A1.md`, whose step-6 list assigns leftovers by owner. A2–A6 run in parallel with
disjoint ownership. Line numbers are from the S1 tree and drift by a line where A1's rewrites change
a line; paths are written as they are after A1.

Common addenda: `. <scratch>/env.sh` first in every shell call; one build, test or gate per shell
call into `<scratch>/logs/<ID>-*.log`; files outside your ownership are foreign
(the two `.rdb` files are listed in `<scratch>/foreign_baseline.sha256`); never hand-rewrite a spelling the relocation tool
rewrites; new or changed tests run twice; a perturbation proof for every check whose logic you
change; A2–A5 update the README of every folder whose code they change, A6 every other document.

## A2 — xtask deploy, staging and db (M)

Owned: `tools/xtask/src/commands/deploy/**`, `tools/xtask/src/commands/staging/**`,
`tools/xtask/src/commands/db/**`, `tools/xtask/src/commands/setup/**`,
`tools/xtask/src/verifications/deployment/**`, `tools/xtask/src/verifications/api_readiness/**`,
`tools/xtask/src/core/repository_layout.rs`, and in `tools/xtask/src/commands/ci/task_definitions.rs`
only the `rust-test-it` task (F-018); the READMEs of those folders.

Work:
1. **Host agent names (D17).** A1's rows renamed every code site (unit template
   `fleet_host_agent@.service`, `deploy/staging/host_agent.rs:55-58,63`, `fleet_instances.rs:350`,
   `fleet_units.rs:28,52,59,65`, `staging/staging_settings.rs:181`,
   `environment_identity/build_identity.rs:30`, `fleet_procedure/operator_lists.rs:191`,
   `fleet_procedure/waves/mod.rs:44` and their tests); verify them. No host carries
   `fleet-host-agent@N` units (the only staging host still runs the single instance, checked
   2026-10-02), so no five-instance rename migration is needed.
2. **Single-instance migration stays** and becomes the D17 migration: the `.rs` text row rewrote
   the names it reads from the host (`legacy_single_instance_migration.rs:22-23,52`,
   `remote/fleet_deploy.rs:45,55`, `tests/legacy_single_instance_migration/tests.rs:40,60,72`,
   `tests/remote/tests.rs:191,200`) to snake_case names no host ever had. Restore the kebab-case
   names of what the host has today (`fleet-host-agent.service`, `~/.config/fleet-host-agent/`,
   `~/.local/bin/fleet-host-agent`) in that module and its tests only, keep every name it installs
   snake_case, and make it retire the old configuration folder and binary along with the old
   units. Its module header says why the old spelling lives there. Tests red first where the
   behaviour changes.
3. Staging compose check (`verifications/deployment/staging_compose_paths.rs:89,94`): the tool
   turns `GOOD_PATH` into `deploy/compose.staging.yml` and the deliberately wrong `BAD_PATH` into
   `apps/api/docker-compose.staging.yml`, which never existed. Re-derive the wrong spellings for
   the new layout (`deploy/compose.dev.yml`, any compose file outside `deploy/`) and the regex of
   `staging_compose_paths/source_audit.rs:313,341`; tests `tests/staging_compose_paths/tests.rs`.
   Perturbation proof.
4. Upload move (`deploy/website/remote_steps.rs:184-188`): the tool rewrote the server location
   `{remote_dir}/apps/website/api_v2/uploads` to `apps/api/uploads`, which never held uploads.
   After OC-deploy step W2 confirms the old folder is empty, retire the step and its test.
5. `verifications/api_readiness/fingerprint.rs:63`: drop `"apps/website/.env"` (no row covers
   `apps/website` itself; the folder is gone).
6. `commands/db/operations/ab.rs:180-185`: name the private clone `compose.dev.yml` like its
   source.
7. Check the tool's rewrites in owned files: `repository_layout.rs:9-29`,
   `deploy/website/rsync_argv.rs:39-42`, `deploy/staging/remote/ssh_argv.rs:35-39,50`,
   `deploy/website/remote_steps.rs:85-86,115,133,166`, `deploy/website/help_text.rs:45-47`,
   `staging/staging_settings.rs:144`, `setup/staging_server.rs:116`,
   `db/milestone_announcement.rs:4,214`.
8. **F-018**: `rust-test-it` (`commands/ci/task_definitions.rs:459-485`) runs
   `podman exec tbd_reforger_db …` four times, so it fails where `podman` is reached through the
   distrobox bridge, while `cargo xtask db test-it` resolves the runtime through
   `commands/db/operations.rs:334` (`runtime()`). Make the task run `cargo xtask db test-it`
   (same fresh `rust_it` database and reaper) instead of re-implementing it; add a test that no
   ci task script names a bare container runtime, with a perturbation proof.

Green: `cargo test -p xtask deploy`, `staging`, `db`, `verifications::deployment`,
`api_readiness`, `ci::` (each its own log); `cargo clippy -p xtask --all-targets --locked -- -D warnings`;
`cargo xtask deploy website --dry-run`; `cargo xtask deploy staging --dry-run` and the same with `--migrate-single-instance` (shows the
migration with the kebab-case sources and snake_case targets); `cargo xtask db up` then `cargo xtask db test-it --test contract_parity_goldens`;
`cargo xtask refactor relocate --verify`.

Budget: M (250k tokens).

## A3 — wave execution (M)

Owned: `tools/xtask/src/commands/platform/**`, `tools/xtask/src/core/cargo_target_directory.rs`,
`tools/ticket_engine/src/repository.rs` and its tests; the READMEs of those folders.

Work:
1. Sparse checkout sets (`ticket_engine/src/repository.rs:112-125`): the `website` target names
   `apps/website` and `documentation::WEBSITE_DOCUMENTATION_DIR` (`:143-145`,
   `documentation/website`); both are gone. A `website` slice needs `apps/api`, `apps/frontend`,
   `apps/offline_service_worker`, `legacy`, `crates`, `deploy` and the mirrors
   `documentation/apps`, `documentation/legacy`; make sure it also gets
   `contracts/fixtures/api_goldens` (today only the `shared` target carries `contracts`). Replace
   the website documentation constant with the two mirror roots; update the tests that pin sets.
2. Orphan fragments: `wave_execution/changed/changed_rs.rs:21,382` and `touch.rs:267-269` use
   `apps/website/shared/*.rs` as the example of a Rust file with no package ancestor; the table now
   sits inside `crates/foundation/http_url_guard/`. Keep the generic walk, rewrite the examples,
   and keep a test for a file with no package ancestor.
3. Spellings the text rows cannot match: `touch.rs:113` (`libwebsite_map_engine-<hash>.rmeta`),
   `trunk.rs:19` (`website-frontend_bg.wasm`), `changed/changed_rs.rs:299`,
   `gate/gate_dispatch.rs:244`, `test_cmd.rs:5` (`website_frontend-<hash>`).
4. Check the tool's rewrites: `changed.rs:13`, `trunk.rs:63,85`, `touch.rs:318-352` (match arms on
   `"frontend"`, `"map_engine" | "graphics_engine"`), `gate/gate_dispatch.rs:148-269`, `db.rs:307`,
   `flush.rs:230`, `mod.rs:113-118`, `test_cmd.rs:66-107`,
   `changed/include_consumer_package_dirs.rs:103,117`, `migrate/gate_db_migrate_claim_body.rs:23`.
   Gate subfolder names (`cargo_target_directory.rs:122-137`) name steps, not packages: keep them.
5. The wasm scope walk (`changed_rs.rs:144-156`) follows the frontend's `path =` dependencies into
   `legacy/` and `crates/`; add a scope test case for a `crates/` member.

Green: `cargo test -p xtask platform`, `cargo test -p ticket_engine` (logs);
`cargo clippy -p xtask -p ticket_engine --all-targets --locked -- -D warnings`;
`cargo xtask ticket check`; `cargo xtask refactor relocate --verify`.

Budget: M (250k tokens).

## A4 — Dockerfile, systemd, env example, runtime fallbacks, Caddy mount (S)

Owned: every tracked file under `deploy/`; `apps/api/.env.example`;
`apps/api/src/core/http_router.rs`; `apps/api/src/core/configuration/**`;
`apps/api/tests/forwarded_for_trust.rs`; a new root `.dockerignore`.

Work:
1. **Dockerfile (D18).** The trimmed workspace (`deploy/Dockerfile`, S1-base lines 19-40) holds only
   `[workspace]`, `resolver` and three members, while the API manifest inherits
   `edition`, `rust-version`, `license` and `publish` from `[workspace.package]`
   (`apps/api/Cargo.toml:4-8`) and members take dependencies and lints from
   `[workspace.dependencies]` and `[workspace.lints]`; the API's path dev-dependency
   `verification_core` (`Cargo.toml:84`) is not copied. Run `podman build -f deploy/Dockerfile .`
   first and record the red. Repair: the synthesised root carries the real root's
   `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]` tables (generated
   from the root `Cargo.toml` at build time, never a hand-kept copy), the member list includes
   `tools/verification_core`, and its folder is copied; or the build uses the real root manifest
   with every member manifest and a `.dockerignore` narrowing the context. Either way `--locked`,
   and the compile-time inputs `contracts/definitions` and `contracts/rules/kit-aliases.json`
   stay. Header comments (lines 1-12) name the new paths.
2. **Caddy mount** (`deploy/compose.staging.yml`, S1-base lines 47-67): the tool re-relativises
   the mount of the old deploy folder (line 63) onto `deploy/` itself, which now also holds
   `deploy.env` (host secrets) and the Dockerfile. Mount only `./Caddyfile` read-only and point
   `command` (line 56) at it. Check the frontend mount (64) against the Caddyfile's `root`, the
   build `context` (75), `dockerfile` (76) and the asset mounts (111-112).
3. **Runtime fallbacks**: `http_router.rs:109-117` and `configuration/mod.rs:32,152` resolve
   `../../../assets/…` against the working directory, right only when the API runs from its crate
   folder; the tool rewrites them one level shallower. Verify each literal and comment,
   `.env.example` (13 such defaults), and `deploy/systemd/tbd-website-api.service:12-15`.
4. Check the tool's rewrites: `tbd-website-api.service:42,48,51,54-55`, the backup units' install
   lines (`tbd-website-backup.service:12-14`, `tbd-website-backup-drill.service:14-16`),
   `fleet_host_agent@.service:1,6,16`, `deploy.env.example:53,109,134-138`, `deploy/Caddyfile:3`,
   `configuration/tests/configuration.rs:241-254`, `forwarded_for_trust.rs:398-399`.
5. D17 prose in `deploy/systemd/README.md` (10 mentions) and `deploy/README.md`; keep the glossary
   anchor `#fleet-host-agent`.

Green: `podman build -f deploy/Dockerfile .` (log); `podman compose -f deploy/compose.staging.yml config`
(log); `cargo test -p api --lib core::configuration` (log; a filter that selects no test is a
failure); `cargo xtask db test-it --test forwarded_for_trust --test map_assets_rate_limit_exemption`;
`cargo clippy -p api --all-targets --locked -- -D warnings`; `cargo xtask deploy website --dry-run`.

Budget: S (150k tokens).

## A5 — `http_url_guard` and `offline_cache_policy` born (M)

Owned: `crates/**`; `apps/offline_service_worker/**`; in `apps/frontend/src/v2/`: `core/offline/**`,
`core/auth/url_guard.rs`, `core/auth/tests/url_guard.rs`, the `pub mod url_guard;` line of
`core/auth/mod.rs`, every file that imports the URL guard or the worker library (list them with
`git grep -n -E "url_guard|offline_service_worker::"`; known: `pages/mission_hub/library/card_grid.rs:32,40`,
`pages/command_center/announcements/article_viewer.rs:117`,
`pages/operations/deployments/service_record.rs:135`,
`pages/field_tools/mortar/tests/catalog_source.rs:5`) and the seven page tests below; in
`apps/api/`: `src/core/text/**`, the five callers of `is_http_url`
(`community_content/handlers/announcements_admin.rs:58`,
`identity_and_access/services/account_registration.rs:77`,
`match_telemetry/models/match_results_revision.rs:207`,
`operations/services/event_authoring/event_creation.rs:137`,
`missions/validation/mission_fields.rs:17`), `tests/aar_replay_url_backfill.rs`,
`tests/cms_url_guard.rs`; root `Cargo.toml`, `Cargo.lock`, `apps/api/Cargo.toml`,
`apps/frontend/Cargo.toml`; `documentation/restructure/manifests/s2_crate_births.tsv`.

**Why a second manifest:** the births need compile glue (two new manifests, workspace members,
`mod` lines, merged predicates) that A1, a mechanical agent, must not write, and A1 must leave a
compiling tree. Every file A5's rows touch is A5's: `git grep` finds the moved files named only in
their own folders' READMEs, the frontend manifest and `crate_catalogue.md` (which abbreviates the
paths, so the tool does not read them). Step 1 is the dry run; if it lists any rewritten file
outside A5's ownership, stop and report.

### Manifest draft (`s2_crate_births.tsv`, paths after A1)

```text
kind	from	to	scope
# http_url_guard: the API's copy of the predicate and its tests become the crate's module
path	apps/api/src/core/text/http_url_guard.rs	crates/foundation/http_url_guard/src/http_url.rs
path	apps/api/src/core/text/tests/http_url_guard.rs	crates/foundation/http_url_guard/src/tests/http_url.rs
rust_path	crate::core::text::http_url_guard::	http_url_guard::	apps/api
rust_path	crate::v2::core::auth::url_guard::	http_url_guard::	apps/frontend
# offline_cache_policy: the worker library's policy modules and tests
path	apps/offline_service_worker/src/lib.rs	crates/contracts/offline_cache_policy/src/lib.rs
path	apps/offline_service_worker/src/README.md	crates/contracts/offline_cache_policy/src/README.md
path	apps/offline_service_worker/src/cache_names.rs	crates/contracts/offline_cache_policy/src/cache_names.rs
path	apps/offline_service_worker/src/network_fallback.rs	crates/contracts/offline_cache_policy/src/network_fallback.rs
path	apps/offline_service_worker/src/offline_pack.rs	crates/contracts/offline_cache_policy/src/offline_pack.rs
path	apps/offline_service_worker/src/request_classification.rs	crates/contracts/offline_cache_policy/src/request_classification.rs
path	apps/offline_service_worker/src/tests/cache_names.rs	crates/contracts/offline_cache_policy/src/tests/cache_names.rs
path	apps/offline_service_worker/src/tests/network_fallback.rs	crates/contracts/offline_cache_policy/src/tests/network_fallback.rs
path	apps/offline_service_worker/src/tests/offline_pack.rs	crates/contracts/offline_cache_policy/src/tests/offline_pack.rs
path	apps/offline_service_worker/src/tests/request_classification.rs	crates/contracts/offline_cache_policy/src/tests/request_classification.rs
# the worker binary becomes the package root; range slicing stays with it
path	apps/offline_service_worker/src/bin/offline_service_worker/main.rs	apps/offline_service_worker/src/main.rs
path	apps/offline_service_worker/src/bin/offline_service_worker/cached_range_response.rs	apps/offline_service_worker/src/cached_range_response.rs
path	apps/offline_service_worker/src/bin/offline_service_worker/fetch_handling.rs	apps/offline_service_worker/src/fetch_handling.rs
path	apps/offline_service_worker/src/bin/offline_service_worker/lifecycle_events.rs	apps/offline_service_worker/src/lifecycle_events.rs
path	apps/offline_service_worker/src/bin/offline_service_worker/worker_scope.rs	apps/offline_service_worker/src/worker_scope.rs
rust_path	offline_service_worker::range_slicing::	crate::range_slicing::	apps/offline_service_worker
rust_path	offline_service_worker::	offline_cache_policy::
```

Rows: 17 `path`, 4 `rust_path`. Each `from` exists after A1 (S1-tree equivalents under
`apps/website/api_v2/` and `apps/website/offline-service-worker/` verified); no `to` exists. The
bin READMEs (`src/bin/README.md`, `src/bin/offline_service_worker/README.md`) cannot move onto
`src/README.md` (a `to` must be absent before the moves): after the apply A5 writes a new
`apps/offline_service_worker/src/README.md` from the bin README's text and deletes both. Check in
the dry run that the specific `offline_service_worker::range_slicing::` row wins over the general
one; if the tool applies them in another order, scope the general row to `apps/frontend` and add a
second one for the worker.

### Crate anatomy (D2; laws_and_gates.md crate anatomy and crate tiers)

| Crate | Files | Manifest |
|---|---|---|
| `crates/foundation/http_url_guard` | `Cargo.toml`, `README.md` (moved by A1, rewritten: Contents, role, boundaries, the field list of who calls it moves to the API's `core/text/README.md`), `src/lib.rs` (≤ 80 lines), `src/prelude.rs` (`is_http_url`), `src/http_url.rs` (the predicate; doc names no API or page internals), `src/cases.rs` (`IS_HTTP_URL_CASES`, `#[cfg(any(test, feature = "test_fixtures"))] pub mod cases`; its header rewritten, it names files that do not exist), `src/tests/http_url.rs` (the API's tests moved, plus the frontend's `core/auth/tests/url_guard.rs` tests merged under their own names so the test-name census holds) | `name = "http_url_guard"`; edition, rust-version, license, publish and lints from the workspace; `url = { workspace = true }`; `[features] test_fixtures = []`; `[package.metadata.layout] category = "foundation"`, `tier = 0`, `targets = "any"` |
| `crates/contracts/offline_cache_policy` | `Cargo.toml`, `README.md` (new), `src/lib.rs` (moved, cut to the anatomy), `src/prelude.rs`, `src/README.md` (moved), the four modules and four tests; `error.rs` only if a public function returns a fallible result | `name = "offline_cache_policy"`; workspace fields and lints; the external crates the four modules use (`serde`, `serde_json`, `url`, all `workspace = true`); `[package.metadata.layout] category = "contracts"`, `tier = 0`, `targets = "any"` |

Also: `crates/README.md`, `crates/foundation/README.md`, `crates/contracts/README.md` (Contents
blocks); root `Cargo.toml` members gain `crates/*/*` (rule 1 of crate tiers: every manifest under
`crates/` is a member) and `[workspace.dependencies]` entries for both crates.

### Consumers switched

- The predicate: the API's five callers (rows rewrite `crate::core::text::http_url_guard::`);
  `core/text/mod.rs:7` loses `pub mod http_url_guard;`. The frontend's callers import
  `http_url_guard::is_http_url` (rows rewrite full paths; leaf imports such as
  `use crate::v2::core::auth::url_guard;` with `url_guard::is_http_url(…)` are edited by hand);
  delete `core/auth/url_guard.rs` and `core/auth/mod.rs:21` after proving the two copies agree on
  all 86 cases (they are byte-identical bodies today: frontend `url_guard.rs:66-76`, API
  `http_url_guard.rs:62-79`).
- The 10 `include!` sites become `use http_url_guard::cases::IS_HTTP_URL_CASES;` with
  `http_url_guard = { workspace = true, features = ["test_fixtures"] }` under `[dev-dependencies]`
  of the API and the frontend: API `src/core/text/tests/http_url_guard.rs:106` (moved into the
  crate by the row, so it becomes the crate's own module) and `tests/aar_replay_url_backfill.rs:46`;
  frontend `core/auth/tests/url_guard.rs:7-10` (merged into the crate),
  `pages/account/settings/tests/settings.rs:5-8`,
  `pages/command_center/announcements/tests/announcements.rs:50`,
  `pages/mission_hub/library/tests/mission_library.rs:174`, `pages/navigation/tests/layout.rs:32`,
  `pages/operations/deployments/tests/deployments.rs:14`,
  `pages/operations/event_detail/tests/event_hub.rs:506`,
  `pages/operations/leaderboards/tests/leaderboards.rs:128`.
- The policy: the frontend's dependency (`apps/frontend/Cargo.toml:36`) becomes
  `offline_cache_policy`; the worker drops `[lib]` and `[[bin]]` (its binary keeps the default
  name `offline_service_worker`, so `index.html:17` and `service_worker.js:7-8` stand), depends on
  `offline_cache_policy`, and `main.rs` gains `mod range_slicing;`.
  `core/offline/saved_copies.rs:21` re-exports `network_fallback` items: import them where they
  are used instead.
- App bundle name: Trunk now emits `frontend-<hash>.js` / `_bg.wasm`; fix
  `core/offline/service_worker_registration.rs:18-20` (`APP_BUNDLE_PREFIX`), the tests
  `core/offline/tests/offline_manifest.rs:23-40`, `offline_pack.rs:53`,
  `service_worker_registration.rs:12-35`, the policy test `request_classification.rs:31` and
  `core/offline/README.md:55`; prove it on a release build.

### How the strangler ledger ends empty

The strangler law (`tools/verification_core/src/repository_laws/workspace_laws/strangler.rs`)
reports dependency edges onto `legacy/` (apps and `tools/xtask`, `tools/developer_tools` excepted)
and any `pub use` of a workspace crate outside `legacy/` inside a `legacy/` member. A5's sources are
apps, not legacy, and the switch happens in the same agent, so no shim is ever written: the old
modules are deleted, not turned into re-exports, and the anatomy law forbids `pub use` of another
workspace crate outside a prelude. Proof: `cargo xtask verify strangler`,
`cargo xtask verify crate-anatomy`, and `git grep -n -E "pub use (http_url_guard|offline_cache_policy)"`
finding nothing outside the two preludes.

Green: `cargo test -p http_url_guard` and `cargo test -p offline_cache_policy` (each twice);
`cargo test -p frontend url_guard`, `cargo test -p frontend offline`,
`cargo test -p frontend settings`, `cargo test -p api --lib core::text` (logs);
`cargo xtask db test-it --test aar_replay_url_backfill --test cms_url_guard`;
`cargo clippy -p http_url_guard -p offline_cache_policy -p offline_service_worker --all-targets --locked -- -D warnings`;
`cargo clippy -p offline_service_worker --target wasm32-unknown-unknown -- -D warnings`;
`cargo xtask mk ci-local-leptos` (the Trunk release build proves the glue names);
`cargo xtask verify crate-anatomy`, `cargo xtask verify crate-tiers`, `cargo xtask verify strangler`;
`cargo xtask refactor relocate --verify` (both S2 manifests).

Budget: M (250k tokens).

## A6 — documentation (S)

Owned: every tracked `.md` outside the folders whose code A2–A5 change (A2: the xtask `deploy`,
`staging`, `db`, `setup` command folders and the `deployment` and `api_readiness` verification
folders; A3: `tools/xtask/src/commands/platform/`, `tools/ticket_engine/src/`; A4: `deploy/`;
A5: `crates/`, `apps/offline_service_worker/`, `apps/frontend/src/v2/core/{offline,auth}/`,
`apps/api/src/core/text/`); `CLAUDE.md` (`AGENTS.md` is a symlink to it: edit `CLAUDE.md` only);
`.cursor/rules/*`; `documentation/apps/api/verification_evidence/requirements.json`.

Work:
1. README Contents blocks and new READMEs: `apps/README.md` (the `website/` entry, the crate list
   at :46, `cargo test -p fleet-host-agent` at :65), a new `legacy/README.md` (the parking folder,
   D12, D13, deleted in S8), `documentation/README.md` (map: `apps/`, `legacy/`),
   `documentation/apps/README.md` (moved website docs index; rewrite its body),
   a new `documentation/legacy/README.md`, `contracts/fixtures/README.md` (`api_goldens/`),
   `documentation/restructure/manifests/README.md` (the two S2 manifests; the tool now treats
   that README as live and rewrites its links), `apps/api/README.md`, `apps/frontend/README.md`
   (no `tests/` folder any more: lines 19, 124, 148-156), `apps/api/seeds/README.md:46,121`.
2. `CLAUDE.md`: §2 atlas (the website subtree becomes `apps/api`, `apps/frontend`,
   `apps/offline_service_worker`, plus `legacy/`, `crates/`, `deploy/`; the xtask `deploy/` line
   goes; `documentation/apps/`, `documentation/legacy/`; arrows aligned), §1.6 layer names, §1.9
   paths, §3 commands and the `apps/api/.env` configuration path. `.cursor/rules/tbd-platform.mdc:15`.
3. D17 prose: every Markdown naming the host agent's package, binary, units, configuration folder
   or user agent (the 32 files that link the glossary anchor, the runbooks under
   `documentation/runbooks/game_server_staging/` and `staging_verification/`, the glossary entry in
   `documentation/glossary/a_to_f.md`, `documentation/apps/fleet_host_agent/*`,
   `apps/fleet_host_agent/README.md`, `apps/fleet_host_agent/src/README.md`) and the 4 entries of
   `requirements.json`. Keep every `#fleet-host-agent` anchor: the heading stays "Fleet host agent".
4. Single-instance runbook paragraphs, if Q1 is accepted: `game_server_staging/README.md:70`,
   `host_preparation.md:275`, `machine_credentials_and_mission_deployment.md:73`,
   `staging_deploy.md:40,143,151,176,178` give way to the host agent name migration.
5. `documentation/restructure/target_file_tree.md`: drop `apps/frontend/tests/fixtures/` (Q4);
   `documentation/architecture/workspace_layout.md`: the workspace after S2, with the two
   scheduled mirror exceptions (frontend inner layout until S3, the mod mirror until M1).
6. Runbooks: `website_deployment.md` and `local_development.md` (`apps/api/.env`, `cd apps/api`,
   the Caddy mount), and every link to the deleted `apps/website/README.md` or the gone
   `documentation/website/`.

Green: `cargo xtask ci verify-documentation`; `cargo xtask ticket check --strict`;
`cargo xtask refactor relocate --verify`; `cargo xtask ci verify-editorconfig`.

Budget: S (150k tokens).

## OC-deploy: operator steps

Local checkout (after A1, before the S2 commit):
- L1. `git status --ignored --short apps deploy tools/xtask`: `apps/api/.env`, `apps/frontend/dist/`
  and `deploy/deploy.env` at their new homes (they ride with whole-folder `git mv`); if one did
  not, move it by hand. The ignore rule (S1 `.gitignore:16`) becomes `deploy/deploy.env`.

Staging host: checked by the orchestrator on 2026-10-02 (read-only): the single instance is still
installed (`fleet-host-agent.service`, `tbd-reforger.service`, `~/.config/fleet-host-agent/`), and no
`fleet-host-agent@N` unit exists.

Website host, before the first post-S2 `cargo xtask deploy website`:
- W1. `mkdir -p <repo>/apps/api && mv <repo>/apps/website/api_v2/.env <repo>/apps/api/.env`.
  The rsync runs with `--delete` (`tbd-website-api.service:30`) and its exclude moves to
  `apps/api/.env` (`deploy/website/rsync_argv.rs:40`), so the old file loses its protection and
  the first deploy would delete it.
- W2. Confirm `<repo>/apps/website/api_v2/uploads` is empty or absent (`remote_steps.rs:184-188`).
- W3. After the deploy: re-install `tbd-website-api.service` (now `apps/api` at lines 48, 51; by
  hand per lines 7-8, or through the deploy's rendering of `WEBSITE_API_UNIT`,
  `repository_layout.rs:27-29`), `systemctl --user daemon-reload`, restart; recreate Caddy
  (`docker compose -f deploy/compose.staging.yml up -d caddy`, or `podman compose`); remove the
  orphaned `<repo>/apps/website/` (its excluded `frontend/dist/` survives `--delete`).

Staging host, before the first post-S2 `cargo xtask deploy staging`:
- S1. `mkdir -p <checkout>/apps/api && mv <checkout>/apps/website/api_v2/.env <checkout>/apps/api/.env`
  (`staging/staging_settings.rs:144`, exclude `deploy/staging/remote/ssh_argv.rs:36`); remove the
  orphaned `<checkout>/apps/website/frontend/dist/` (`ssh_argv.rs:50`).
- S2. Host agent names (D17): the first post-S2 `cargo xtask deploy staging --migrate-single-instance`
  stops the kebab-case single instance, retires its unit, configuration folder and binary, and
  installs the five snake_case instances. The operator chooses when to run it; it is not part of
  the S2 gate, whose dry runs show it.

GitHub:
- G1. Required status checks named after renamed ci.yml jobs (`.github/workflows/ci.yml:141-142`,
  the API job) need updating in the branch protection rule.

## Gate extras for S2 (checkpoint stage)

The full gate set GS of [laws and gates](/documentation/archive/restructure/laws_and_gates.md#standard-gate-set)
(items 1–15), one command per shell call, plus:
- `podman build -f deploy/Dockerfile .` (D18) and `podman compose -f deploy/compose.staging.yml config`;
- `cargo xtask deploy website --dry-run` and `cargo xtask deploy staging --dry-run` (the host agent
  name migration line present);
- `cargo xtask refactor relocate --verify` over both S2 manifests;
- the `Cargo.lock` diff: the six renamed packages, their dependents and the two new crates only;
- crate tiers, crate anatomy and strangler hard on the two new crates; every package name equals
  its folder name;
- the wasm size probe (GS 15) reads `frontend-<hash>_bg.wasm`;
- `git grep` for `apps/website`, the `website-`/`website_` package spellings, `fleet-host-agent`
  and `Caddyfile.website` outside the frozen records: each remaining hit is an anchor, the host
  agent name migration module, or has a verdict in a report.
