**Status:** archived — see [the restructure program](/documentation_v2/restructure/README.md)

# Improved layout claim check

I checked every concrete claim in the 20 spec files against the code. The website specs get the location facts right but are wrong on numbers and on the two biggest proposals: the DTO deletion and the test merge. The mod specs invent the whole objectives layout, and the scripts-flattening plan misses tooling that depends on the `TBD/` path prefix.

## Website

| Claim | Verdict | Evidence |
|---|---|---|
| `apps/website/Dockerfile`, `apps/website/docker-compose.staging.yml`, `apps/website/api_v2/docker-compose.yml` exist | TRUE | All three on disk |
| xtask files that reference them | — | Staging compose: `verifications/deployment/staging_compose_paths.rs:89` (`GOOD_PATH`), its `tests/staging_compose_paths/tests.rs` (many lines), `commands/deploy/website/remote_steps.rs:85-86`, `deploy/website/help_text.rs:45,47`, `deploy/tests/website/tests.rs:333-334,394` (`include_str!` of the compose file), `deploy/Caddyfile.website:3`, `deploy/README.md`, `deploy.env.example:134-140`. Dev compose: `commands/db/operations.rs:23,125` (`WEB` = `apps/website/api_v2`, then `cd {web} && compose …`), `db/operations/selftest.rs:62-79`, `db/operations/ab.rs:180`, `db/README.md:41`. Dockerfile: only `deploy.env.example:140` and docs |
| The staging deploy runner `ssh_argv.rs` "lines 217 & 221" run compose | FALSE | The file has 124 lines and runs no compose command. The gate requires that the game-server deploy runs none (`staging_compose_paths.rs:4,80-83`) |
| `remote_steps.rs` "line 32 `compose_up`" | FALSE | Line 32 is the `STAGING_DB_CONTAINER` doc comment. The function is `compose_session` (about lines 81-87) |
| `staging_compose_paths` constants as quoted | TRUE | `staging_compose_paths.rs:89,94` |
| Staging compose path updates are complete | PARTIAL | The spec misses the `caddy` service and its mount `../../tools_v2/xtask/deploy` (compose:63). Spec 08:56 sets `dockerfile:` to `deploy/Dockerfile.api`, which is wrong because the build context is the repo root; spec 03 has it right |
| `db up` becomes `compose -f …dev.yml` | PARTIAL / breaks | Seeds are piped as `< seeds/<file>` relative to `api_v2` (`operations.rs:425`, `selftest.rs:75-79`), so dropping the `cd` breaks the seed paths. The selftest frozen recipe strings also need updating |
| Frontend `core/api/dto/` hand-duplicates wire DTOs | PARTIAL | 22 files at the top level (20 `.rs` plus `README.md` and `mod.rs`), 8 in `equipment_data_viewer/`, 19 test files in `dto/tests/`. 47 `.rs` files, 7,476 lines in total |
| "15+ parity test files" | TRUE | 19 |
| API generates types with typify from `contracts_v2/definitions` | TRUE, wrong location | `tools_v2/xtask/src/commands/generate/schema_types.rs:23-145`. 30 schemas write 266 files into `api_v2/src/<domain>/models/generated/` (and `missions/contract/generated/`), all inside the `website-api` crate |
| Frontend DTOs are identical in shape to the API models | FALSE | The DTO README says they mirror the hand-written `api_v2/src/<domain>/models/` (not the typify output) and deliberately differ: extensible enums travel as `String`; `#[serde(flatten)] extra` catch-alls (`missions.rs:57,219,229`, `events.rs:50`, `content.rs:42`, `match_events.rs:41`); empty-string defaults; tri-state `absent_null_or_value` patches; `MatchEventDetail` decoding; frontend-only methods (`MissionDetail::compiled_meta` → map-engine, `servers.rs:92` `decode_server_status_frame`); and imports of frontend types (`crate::v2::core::auth::{Role, User}` in `administration.rs:22`, `auth.rs:16`) |
| What the "R-api golden test parity" does | — | `dto/tests/r_api.rs` deserializes captured live-API goldens (`frontend/tests/fixtures/api/`, `contracts_v2/fixtures/…`) into each DTO and checks the re-serialization is canonically byte-equal. A second half poisons values to prove every wire key is claimed by a named field and not swallowed by `flatten`. It is a drift detector against the live API, not a duplicate type check |
| A WASM-friendly `website-api-types` crate (serde, chrono, uuid) is feasible as a move | FALSE as written | 25 of 289 model files use sqlx. There are 63 `sqlx::FromRow` / `sqlx::Type` / `sqlx::types` uses and 15 `#[sqlx(…)]` attributes on the same structs that derive Serialize (e.g. `missions/models/mission.rs:18-214`). `RawJson = sqlx::types::Json<…>` (`core/wire_format/raw_json.rs:5`). The typify output needs `regress` (`Cargo.toml:47-48`), which the spec's dependency list omits. Generated types are mostly used for test round-trips, not as handler response types |
| `core/ui` imports tokens from `apps::editor` | TRUE | `core/ui/slider.rs:19`, `select.rs:15`, `search_box.rs:18`: `use crate::v2::apps::editor::shell::layout::{DISABLED_GLYPH, HOVER_FILL};` |
| `core/auth/store.rs` calls the editor IndexedDB purge | TRUE | `core/auth/store.rs:259`: `crate::v2::apps::editor::shell::hydrate::purge_local_documents(&owner);` (wasm32 only) |
| `pages/navigation/` holds AppLayout, TopNav, Sidebar | TRUE | Files: `layout.rs`, `sidebar.rs`, `top_nav.rs`, `nav_config.rs`, plus `membership_status.rs`, `not_found.rs`, `mod.rs`, `tests/`, `README.md`. The spec lists only four. `tests/layout.rs:32` has a relative `include!` of `shared/` that would break on a move |
| ~95 flat test files in `api_v2/tests/` | FALSE | 154 top-level `.rs` files, 22 support directories, 236 `.rs` files in total |
| No reason the tests are separate binaries | FALSE | `tests/common/database.rs:104-200` gives each binary its own database, `<base>_<CARGO_CRATE_NAME>_it`, specifically so one suite's leftover rows cannot affect another's results. Failpoints are a process-global registry (`failpoint_and_race_support/mod.rs:12-13`). No `sqlx::test` is used. Merging binaries would merge databases and failpoint state |
| Empty untracked `frontend/src/v2/map_engine/` | FALSE | Not on disk, nothing tracked |
| Gates `engine-layers` and `staging-compose-paths` exist | TRUE | `commands/verify/cli.rs`. `readme-coverage`, `markdown-placement`, `link-check`, `file-length`, `faction-library-seeds`, `wiki-seeds` and `ci verify-codegen-fresh` also exist |
| Engine rules 4 and 7 as described | FALSE (swapped) | The spec's Rule 4 text is the real Rule 7 and vice versa (`documentation_v2/standards/engine_boundary_rules.md:311,346`) |
| 39 migrations; `connection_pool.rs` runs `sqlx::migrate!` | FALSE | 58 SQL files, up to `0061`. The macro is in `core/database/mod.rs:76` |
| 5 seeds | PARTIAL | `db seed` applies 5, but `seeds/` holds 8 SQL files |
| `shared/` is a single-file dir with an 86-case table, `include!`d by both crates | PARTIAL | `README.md` plus the table; 86 cases is correct. Included at `api_v2/src/core/text/tests/http_url_guard.rs:106` and 8 frontend test sites |
| Website root listing is complete | FALSE | Omits the `apps/website/offline-service-worker` crate (workspace member, `Cargo.toml:13`) |
| `core/` listing is complete | PARTIAL | Omits `core/map_view/` and `core/offline/`. The `core/api` listing omits `audit_stream/`, `sse_frames.rs`, `client/rate_limit_retry.rs`, `endpoints/server_registry.rs` |
| `aar/` and `planner/` are README-only stubs not in `apps/mod.rs` | TRUE | |
| Trunk and `.gitignore` claims | TRUE | `Trunk.toml:3,8,18,33`; `.gitignore:54-55` |
| Rename churn | — | 299 frontend files use `core::api`; 321 lines name `api::dto` |

## Mod

| Claim | Verdict | Evidence |
|---|---|---|
| `crf_framework` and `vanilla_reference` sit at `apps/mod/` root and are gitignored | TRUE (gitignore), absent on disk | `.gitignore:64-65`, which also lists `playable_selector` and `Tbd_framework` (the spec leaves `playable_selector` out of `References/`) |
| Tooling that references the reference folders | PARTIAL list | Spec misses: rsync excludes in `deploy/website/rsync_argv.rs:57-58` and `deploy/staging/remote/ssh_argv.rs:29-30`, `staging/remote.rs:25`, `verification-core/.../source_roots.rs:36`, `ui_layouts.rs:32`. Listed and present: `upstream_code_leaks.rs:92` (`CRF_REL`; it also scans `tbd-export` at :90 and vanilla paks via `$HOME` at :99), `slice_worktree/git_plain.rs` (11 references), `fetch/vanilla_{api,source}.rs`, `enfusion_tooling/{carve,cli,mod}.rs` |
| Fetch commands "extract vanilla scripts from Steam game files" | FALSE | They mirror web pages: Doxygen HTML into `vanilla_reference/apidoc/` and arexplorer pages into `vanilla_reference/source_html/`. Extraction from paks is `enf carve` (`carve.rs:285`) |
| CRF is "GPL-licensed" | FALSE | Arma Public License (`upstream_code_leaks.rs:31`) |
| `apps/mod/` root holds `.mcp.json` and `.local-test-profile/` | FALSE in this checkout | Root has only `README.md`, `improved_layout/`, `tbd-emcp/`, `tbd-export/`, `tbd-framework/`, with no hidden entries. `.mcp.json` was untracked in `9574714e` and is gitignored (`.gitignore:28`). `.local-test-profile/` is gitignored (`.gitignore:70`) and created by `setup/server_profile.rs:127` |
| `.mcp.json` "executes `cargo xtask mcp`" | FALSE | It ran `npx -y enfusion-mcp` (T-1095, `documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md:121`) |
| `Scripts/Game/TBD/` subfolders | PARTIAL | Actual `.c` counts: API 33, Core 17, Gamemode 38, Session 115, Systems 143, UI 41 (387 total). The spec omits `API/MatchTelemetry` and `Systems/MatchEvents`. The spec's UI list (Admin, Common, Core, Hud, Lobby, Navigation, Screens) is wrong: the actual folders are Common, Core, Hud, Mock |
| Assets reference the `Scripts/Game/TBD` path | FALSE for `.gproj`, `.et`, `.layout`, `.conf`, `.ent`, `.layer` | They bind by class or GUID. But `tbd-framework/resourceDatabase.rdb` holds 213 `Scripts/Game/TBD` path entries, and `tbd-export`'s `.rdb` has more |
| Literal path references in tooling and docs | — | tools_v2: 22 files / 39 lines (11 non-test `.rs`). documentation_v2: 163 files / 373 lines. apps/mod `.md`: 143 files / 722 lines. `.ai`: 234 files |
| `cargo xtask mod compile` is unaffected | FALSE | `compile/report_compile_errors.rs:241` (`count_tbd_warnings`) counts only warnings matching `@"Scripts/Game/TBD/`. After flattening it silently reports 0 (`compile/execution.rs:334,342`) |
| Hard-coded paths in verifications | — | `mod_scripts/{results_reporter_identity_comments.rs:109, mission_rest_size_limits.rs:38, ui_layouts.rs:118, destroy_target_diagnostics.rs:32-36, player_identity_comments.rs:74}` and `schemas/checks/{mission_validation.rs:67, contract_validation/validate_all.rs:374}` |
| `file-length`, `enfusion-comments` and the `enf` index depend on `TBD/` | FALSE | Their roots are `apps/mod/<addon>/Scripts` (`source_roots.rs:31-42`, `enfusion_comments/mod.rs:43-44`) |
| `TBD/` is redundant | FALSE / contested | The repo documents it as the shared-namespace folder (`Scripts/Game/README.md`: "Scripts from every loaded addon share one namespace … sits under `TBD/`"). `tbd-export` uses the same convention (`Scripts/Game/TBD/Export`). The same shadow-avoidance rule is stated for textures (`UI/Textures/README.md:41`) |
| Path collisions if flattened | No collisions vs CRF; vanilla unverifiable | Against `.ai/artifacts/enf-index/crf_files.tsv`, zero file collisions (folders merge, e.g. `Scripts/Game/Systems`). `vanilla_files.tsv` is carved (`Carved/data/NNNNN_hash.c`) with no original paths, and `vanilla_reference` is absent |
| Objectives split into `Model/Registry/Runtime/Tasks` | TRUE | Model: 5 `.c` (`TBD_Objective.c` 276 lines). Registry: 7 (`TBD_ObjectiveRegistry` 377, `RuleResolver` 240, `TypedBinder` 251, `EntityReader` 251, `RulesReader` 156, `DestroyTargets` 138, `EndConditions` 110). Runtime: 3 (`Progression` 354, `ObjectivesComponent` 324, `HudPublisher` 268). Tasks: 4 (`TaskStateMachine` 369, `TaskSchedule` 160, `Task` 45, `TaskStruct` 40) |
| `TBD_ObjectiveCapture`, `TBD_ObjectiveDestroy`, `TBD_ObjectiveHoldUntil`, `TBD_ObjectiveHVT` exist | FALSE | Zero definitions or references. Only `TBD_ObjectiveDestroyTargets` exists (`Objectives/Registry/TBD_ObjectiveDestroyTargets.c`, a static helper) |
| An HVT objective kind exists | FALSE | `TBD_EObjectiveKind` has only `NONE`, `CAPTURE`, `DESTROY`, `HOLD_UNTIL` |
| `TBD_Objective` is a subclassable base with `OnActivated`/`OnTick`/`OnCompleted` | FALSE | It is one data record (`Model/TBD_Objective.c:17`) with no subclasses. Kind dispatch is spread across 11 files through enum checks |
| Gamemode Orchestrator/Stages listing | TRUE | Matches disk |
| `CRF_Rush_Game.c` is over 3,500 lines | TRUE | 3,529 lines |
| Addon GUIDs and dependencies (framework, export→emcp, emcp) | TRUE | The three `addon.gproj` files |

**Where `tbd-missions` appears (to delete):**
- `04_tbd_missions_addon.md`: the whole file.
- `01_master_overview.md`: :27-30, :41, :56-58, :106-111, :126-161 (DAG section, both diagrams), :197.
- `02_mod_root_structure.md`: :33-34, :43, :72-77, :105-112 (§3.2), :150-153, :162. Also "Monolithic … (runtime + scenarios + worlds)" at :25.
- `07_tooling_and_ci_impact.md`: :15, :43-46, :49-62 (§2.2), :64-69 (§2.3), :76, :85-90 (§2.6), :92-96 (§2.7), :104-105, :115.
- `08_migration_roadmap.md`: :21 and all of Phase 2 (:59-76), :144.
- `documentation_v2/mod/improved_layout/README.md`: :15, :35, :49-50.
- `apps/mod/improved_layout/README.md`: :46-50, :72-73.

## Claims that are wrong or would break things

1. **Deleting the frontend DTOs and importing `website-api-types`** would silently change the frontend's wire behaviour. The DTOs use strings for extensible enums, `flatten` catch-alls, empty-string defaults and tri-state patches, and import frontend auth types. The API models carry sqlx derives and `sqlx::types::Json`, so they cannot simply move to a WASM crate. It would also delete the live-golden drift tests, which are not redundant.
2. **Grouping the integration tests into suites** would merge the one-database-per-binary isolation and the process-global failpoint state, which is exactly what `common/database.rs` exists to prevent. The count is also 154, not 95.
3. **Deploy move:**
   - The `ssh_argv.rs` edit would add a compose command to the game-server deploy, which `staging-compose-paths` rejects.
   - Changing `db` to use `-f` without the `cd` breaks the seed paths.
   - Spec 08's `dockerfile:` path is wrong for a repo-root build context.
   - The Caddy volume mount and the `include_str!` in `deploy/tests/website/tests.rs:394` are not mentioned.
4. **Flattening `Scripts/Game/TBD`** makes `mod compile`'s TBD warning count silently 0 and breaks 7 hard-coded verification paths. It also drops the documented namespace and anti-shadowing convention, which `tbd-export` shares. The committed `.rdb` files and about 1,100 doc lines cite the path.
5. **The `Objectives/Types/{Capture,Destroy,HoldUntil,HVT}` plan and the "implement a new type" guide are fabricated.** None of those classes exist, HVT is not a kind, and there is no virtual base class to subclass.
6. **Wrong numbers and facts:**
   - Migrations: 58 files, not 39. Seeds: 8 files, 5 applied.
   - `frontend/src/v2/map_engine/` does not exist.
   - `offline-service-worker` is missing from the root and workspace listings.
   - Engine rules 4 and 7 are swapped.
   - The mod UI subfolder list is wrong.
   - CRF is not GPL.
   - `.mcp.json` is untracked and ran `npx`, not `cargo xtask mcp`.
   - The fetch commands download web pages; they do not unpack game files.
   - `playable_selector` is left out of `References/`.
