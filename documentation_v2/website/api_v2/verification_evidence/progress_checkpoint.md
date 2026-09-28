**Status:** live

# API v2 verification checkpoint — 2026-09-28

Milestones E, F, M, T, C, V and B are implemented and verified; overall readiness is **not
passing**, because staging (S) has not started (see `remaining_milestones.md`) and no receipt
exists: `cargo xtask verify api-readiness --execute` was not run (it needs a quiet working tree).
T is committed on main as 0ef292758 and C as b49fb86c1; V is committed as bd6ec3edf (2026-09-28),
with its fourteen tickets shipped and stamped. B is committed as 55a6eab9f (2026-09-28), with T-940.10, T-1177 and T-1245 shipped and stamped.

## Milestone status

| Milestone | State |
|---|---|
| E — eligibility, allocation, occupancy, no-show, machine credentials, runtime sessions | Complete. |
| F — fleet command ledger, host agent, executors, recovery | Complete. |
| M — artifacts, reviews, approval binding, deployments, authored preservation, workspace | Complete. |
| T — match identity, revisions, corrections, atomicity, detailed events, telemetry queue, fleet dashboard, statistics | Complete (design: `telemetry.md`). |
| C — personnel pagination, audit replay and query recovery, audit frontend, vehicle mutations, wiki features, content storage | Complete, committed as b49fb86c1 (design: `administration_and_content.md`). |
| V — route acceptance, contract parity, properties, controlled races, failure injection, engineering laws | Complete 2026-09-28, committed as bd6ec3edf (design: `verification_completeness.md`; findings: `verification_findings.md`; execution record below). |
| S | Not started (`remaining_milestones.md`). |
| B — game ballistics: flight model, oracle calibration, solver, dispersion, fuze, crest, battery, catalog API, saved fire missions, offline mortar page | Complete 2026-09-28, committed as 55a6eab9f (design: `game_ballistics.md`; execution record below). |

Migrations 0057–0060 are pinned (B adds 0060); the next migration is 0061. Versions 0022–0024 stay retired.

## B in brief

- **Engine.** `map-engine/src/data/scenario/ballistics/`: the flight model is the engine's own
  integrator, identified from 4,185 oracle simulations (fixed 1/30 s step, gravity first,
  quadratic air-relative drag, mean-velocity position, linear crossing; `f32` state as the
  engine); per-ring solver with typed refusals and wind-corrected aim; dispersion as a documented
  interpretation (not verified in-engine); fuze search over charges; crest clearance; battery of
  up to twelve guns; one fire-mission assembler for API and page (`SOLVER_REVISION`
  game-ballistics-2).
- **Calibration.** A tbd-export `BallisticsOracle` plugin and play-mode component export
  forward angles (1.5625-mil lattice) and simulations; `cargo xtask ballistics trim-export`
  writes the vanilla catalog and calibration bundle (31 native and 31 wind tables, every native
  row matched, nothing interpolated), pinned to game build 1.8.0.13. Upload evaluates 7,865
  cases, 0 failures.
- **API.** Migration 0060 (immutable `ballistics_catalogs`, catalog-model fire-mission columns,
  `fire_mission_guns`, the event foreign key with dangling rows detached); catalog list, version
  read (ETag, immutable) and admin upload; `POST /api/v1/fire-missions` re-solves and refuses a
  mismatch over 1 mil or 0.1 s; `POST /fire-missions/solve` retired; list and save follow
  `viewer_event_access`.
- **Frontend.** Mortar page on the map-engine renderer (`core/map_view`) with grid entry,
  per-gun cards, saved fires; admin catalogs page; offline pack through the
  `offline-service-worker` crate (shell, catalog, map pack; saved copies behind a failing API).
- **Register.** Seven B requirements, five map-engine checks on one command (213 cases), two
  gates; minimums at the measured counts (below).

## B verification (2026-09-28; logs in `target/api-progress-checkpoint/2026-09-28-b/`, gitignored)

| Gate | Result |
|---|---|
| map-engine `data::scenario::ballistics::` | 213 passed; full crate 1,687 passed, 0 failed, 2 ignored; clippy native, per feature and wasm32 clean (`f2-map-engine-*.log`) |
| `cargo xtask db test-it` (full) | 1,339 passed, 0 failed, 0 ignored: game_ballistics 29, route_acceptance 73, contract_parity 86, controlled_races 9, failure_injection 27, engineering_laws 10 (`f2-db-test-it.log`) |
| Frontend | `cargo test -p website-frontend` 1,982 passed, 0 failed; fmt clean (`z-frontend-test-2.log`); `mk ci-local-leptos` (the frontend_quality command) exit 0, 1,982 passed |
| `mk leptos-gates` | doctor OK, 21 editor smokes, 26/26 routes match the frozen oracle (`z-leptos-gates.log`) |
| `mk ballistics-wasm-agreement` | PASS 32/32, bit-identical 32/32 (`z-wasm-agreement.log`) |
| `mk mortar-offline-gate` | 15 cases, `mortar-offline: PASS` (`z-mortar-offline.log`) |
| Tool crates | developer-tools 313 passed, 4 ignored; offline crate 28, clippy host and wasm32 clean; xtask 1,058 passed, 1 red on untracked files (prose rule) |
| `ci rust-ci` | run 3 PASS (`z-ci-rust-ci-3.log`): rust-fmt, rust-clippy, rust-build, wasm-ci (with the offline crate) and rust-test-it, 3,098 passed, 0 failed, 2 ignored (the two pre-existing map-engine ignores, unfiltered in wasm-ci); runs 1 and 2 red at rust-fmt, formatted |
| `ci ci-local` | stops at verify-no-python and verify-no-shell only on the 11 unstaged deletions (cleared at commit); every other step PASS run one by one (`z-ci-*.log`); `ci ci-local-schema` PASS, 483 citations |
| Perturbations | orchestrator `p-*.log`, each restored sha256-equal: drag zeroed, Δh ignored, fixture row +5 m, wind sign flipped, comparison tolerances 0, token_type any scheme, bench TOF ×1.01 — each red |
| `verify api-readiness` (judge only, after the register edits; `b28-api-readiness.log`) | The register validates (79 requirements, 98 checks, every implementation path exists); FAIL with no receipts (79 violations, 98 checks did not run), as expected without `--execute` |

## V in brief

- **Route acceptance.** `tests/route_acceptance_support/` reads the route table from the router
  source at test time and holds it against every `@route` tag in both directions. Each route has
  one table-driven specification over seven dimensions (authorized, unauthorized, ownership,
  guest, ban, malformed, boundary); the framework derives every probe that needs no domain
  knowledge. Seven part binaries, `route_acceptance_coverage` and
  `debug_routes_are_development_only` run them.
- **Contract parity.** 19 new response schemas and 123 `@contract` tags in 53 files.
  `contract_parity_goldens` reproduces every frontend golden (89 `_index.tsv` rows) from
  `registry_dev.sql` and `content_golden.sql`, with the 43 normalised fields listed in the design
  note; `contract_parity_mod_wire` checks the mod's DTO fields, wire version and mission schema
  window; the equipment viewer goldens reproduce from a 97 KB committed fixture. Modpacks,
  announcements, CMS announcements and leaderboards decode into typed DTOs.
- **Refusal envelopes.** Every JSON body, query and path rejection answers `{error, details?}`
  with 400, 413 `request_too_large` or 415 (`from_json_rejection`, `from_query_rejection`, and
  `core/http/path_parameters.rs`, which replaces 97 `Path` extractors in 49 handlers). The 12
  `/api/v1/debug/equipment-data/*` routes are registered only in development.
- **Handler defects.** A hidden draft no longer leaks through a peer's bookmark (T-1173);
  nonexistent missions, events, servers and roster accounts answer 404; a negative armory
  quantity, an unknown event scope and a malformed slot id answer 400 (every row in
  `verification_findings.md`).
- **Properties.** Seven properties at 256 generated cases each, seed 2026092201.
- **Failpoints, races, failure injection.** `core/failpoints/` holds 18 named failpoints behind
  the test-only `failpoints` feature, compiled out of the release `api`. Nine controlled-race
  cases cover last seat, refresh, replay, assignment against withdrawal, linking, approval,
  ingest and commit ordering; 27 failure-injection cases cover rollback before commit,
  idempotent or documented retries after commit, and external-effect boundaries.
- **Engineering laws.** `verification_core::repository_laws` holds the file-length,
  test-placement, no-exemption, crate-direction and engine-layer laws; `verify file-length` and
  `verify engine-layers` delegate to it; the frontend `doc_audit` grandfather table is gone
  (T-1041).
- **Readiness fingerprint.** A tracked symlink is fingerprinted by its link text; untracked,
  escaping and dangling links stay refused.
- **Register.** Minimums at the measured counts: route_acceptance 66, contract_parity 85,
  controlled_races 9, failure_injection 27, engineering_laws 10, property invariants 26 (with
  `reservation_transactions_conserve_slots_and_participants` in the pattern), backend_regression
  1,302, frontend_quality 1,826, readiness_self_tests 59. The V requirements name their test
  binaries, support folders, failpoints, path extractor, repository laws, schemas, seed and
  goldens.
- **Findings.** 116 findings triaged in `verification_findings.md`; 23 new idea tickets,
  T-1229 to T-1251.

## V verification (2026-09-28; logs in `target/api-progress-checkpoint/2026-09-28-v/`, gitignored)

| Gate | Result |
|---|---|
| `cargo xtask db test-it` (full) | 1,302 passed, 0 failed, 0 ignored over 148 binaries; tests 222 s, build 46 s; lib 536 (`g-db-test-it-final-2.log`). V prefixes: route_acceptance 66 (coverage 14, seven parts × 7, `debug_routes_are_development_only` 3), contract_parity 85 (goldens 6, mod_wire 6, equipment_viewer 2, `json_rejection_envelopes` 34, `query_rejection_envelopes` 30, seven part responses 7), controlled_races 9, failure_injection 27, engineering_laws 10. Every identity_* and administration_* check at its minimum with every named case |
| Property records | 256/256 at seed 2026092201 for `protected_actions_require_effective_session_authority`, `refresh_replay_revokes_concurrently_issued_successor`, `approval_and_deployment_share_immutable_artifact`, `telemetry_revisions_contribute_exactly_once`, `command_executor_fencing_preserves_observed_outcomes`, `audit_publication_preserves_committed_event_delivery`, `reservation_transactions_conserve_slots_and_participants` |
| Runtime | full `db test-it` inside the 900 s target; register timeout 1,800 s |
| `ci rust-test-it` | exit 0: 1,302 passed, 0 failed over 148 suites on the fixed `rust_it` database (`g-ci-rust-test-it.log`) |
| Frontend | `cargo test -p website-frontend` 1,826 passed, 0 failed; `identity_browser_session_transactions` 9/9 (`g-frontend-test-final.log`); clippy wasm32 exit 0, no warning in a V file; fmt clean; trunk release success |
| `mk leptos-gates` | exit 0, gate doctor OK, editor suite 21/21, DOM oracle 25/25 (`g-leptos-gates-2.log`) after 10 screenshot-audited accepts (`g-accept-*.log`: dashboard, approvals, audit, servercontrol, deployments, events, eventhub, missions, missionview, settings), each tracing to golden content now reproduced from the seed |
| Clippy `-D warnings` | clean: website-api all targets and `--lib --bins` (failpoints off); xtask and verification-core; map-engine and graphics-engine |
| rustfmt | `fmt --check` clean on website-api, xtask, verification-core, developer-tools, map-engine, graphics-engine and the frontend; ci `rust-fmt` PASS |
| Static gates | route-tags PASS 165/165; file-length 0 violations over 4,103 files; enfusion-comments 0; engine-layers PASS; schema-codegen leaves the tree unchanged; ci-local-schema PASS (420 citations); mod compile clean; verify-coding-standards PASS (no-select-star clean); editorconfig, no-node, ci-shell, staging-compose-paths, mission-rest-size-limits and ci-schema-parity PASS; no-python and no-shell FAIL only on V's 7 unstaged deletions (cleared by `git rm` at commit) |
| Tooling tests | xtask `api_readiness` 60 passed (the register pattern matches 59); `property_test_configuration` 8; verification-core 144; developer-tools 269 (4 ignored); full xtask 1,029 passed, 1 red until the new files are committed (`every_rust_file_named_in_prose_exists`) |
| Compile-out | the release `api` carries 0 of the 18 failpoint names and no `failpoint` string; a test binary carries them |
| Documentation (`--with-untracked`) | markdown-placement PASS; link-check 0 breaks (1 check did not run: a deleted tracked README, cleared at commit); readme-coverage 21 violations: 19 in the Workbench-generated `Gameplay/Policy/Generated/` folders (V-F115, T-1250) and 2 cleared at commit |
| `ci ci-local` | stops at step 2 (verify-no-python) on the 7 unstaged deletions (`g-ci-local.log`); every other step run on its own: editorconfig, no-node, ci-shell, engine-layers, rust-fmt, rust-clippy (website-api `-D warnings`), rust-build, wasm-ci (1,493 tests), verify-coding-standards, the ci-local-leptos steps, ci-local-schema, staging-compose-paths, mission-rest-size-limits and ci-schema-parity PASS; rust-test-it as above; verify-documentation judges the index and fails only on V's uncommitted files and V-F115 |
| Perturbations | Orchestrator re-checks (`g-perturb-*.log`), each restored sha256-equal: a golden value → contract_parity red; a 501-line file → engineering_laws red; a removed `@route` tag → route_acceptance red; a failpoint moved after commit → failure_injection red; a duplicate applied → property red. Agent perturbations are in the execution record rows. Not run (the permission classifier refused the edits): X1's race lock removal, Q2, Q4, R6's handler role gate, R4 |
| `verify api-readiness` (judge only, after the register edits; `h1-api-readiness.log`) | The register validates (every implementation path exists) and fingerprinting completes. Verdict `api-readiness: FAIL — 73 violation(s), 91 check(s) DID NOT RUN (of 164)`, exit 2: no receipt exists for any of the 91 checks because `--execute` was not run, so all 73 requirements are unmet. With receipts it would still fail on S (the three operational staging checks) and B (game ballistics) |

## C in brief

- **Personnel.** `GET /api/v1/admin/users?q&page&per_page` answers `{items, page, per_page,
  total}` ordered by `lower(username), discord_id`: `per_page` defaults to 20 and clamps to 100,
  an invalid value is a 400, a page past the end keeps the real total. The personnel page has a
  pager whose page, size and search live in the URL (T-940.7).
- **Audit stream.** Migration 0057 adds the retention floor `retained_after_sequence`, raised by
  delete and truncate triggers. The stream opens with `event: ready`, sends each row with its
  publication sequence as `id`, and answers a cursor ahead of the tail or below the floor with
  `event: reset` (`cursor_ahead`, `history_unavailable`) before continuing from the tail. A publish
  failure still reads; a read failure keeps the cursor and retries on the timer whether or not
  `LISTEN` is healthy.
- **Audit page.** It connects, waits for `ready`, loads the paged history, merges live rows
  deduplicated by audit id, reloads on `reset` and shows a live status badge.
- **Shutdown.** `core/process_lifecycle` holds one process-wide shutdown signal; on SIGINT or
  SIGTERM every open event stream closes, so the graceful drain completes and clients replay after
  their `Last-Event-ID`.
- **Vehicles.** `POST`, `PUT`, `PATCH` and soft `DELETE` share one validator with unknown fields
  refused and a three-state `PATCH`; migration 0058 adds nullable lifecycle columns (legacy rows
  stay null); every mutation appends its audit row in the same transaction. The vehicles page
  gains the administrator form and delete confirmation (T-940.8).
- **Wiki.** The markup service parses `body_md` with pulldown-cmark into typed `blocks` (headings
  H1–H6 with anchors, links, safe images, tables, checklists, seven callout kinds, code, quote,
  rule). A save refuses a stale revision (409), an oversized body (400) and unsafe markup (422 with
  findings). Migration 0059 stores revisions; the page renders the blocks, lists revisions and
  restores one as a new revision (T-940.9).
- **URLs and limits.** `core/text/content_url_policy.rs` decides safe link and image URLs, and
  the frontend's `core/utils/safe_url.rs` agrees with it. `ApiError::from_json_rejection` and
  `from_query_rejection` answer 413 `request_too_large`, 415 and 400 in the standard envelope.
  Uploads answer 413, 400, 415 and 503 `storage_unavailable` and are renamed into place whole.
  Announcement pages break ties by `id DESC`; system audit rows stamp `created_at`.
- **Contracts.** Five schemas (`personnel-roster`, `audit-log`, `vehicle-database`, `wiki-page`,
  `content-upload`) with generated models; frontend DTO modules `administration`, `vehicles` and
  `wiki` with R-api goldens; a deterministic DOM-oracle audit stream fixture.

## C verification (logs in `target/api-progress-checkpoint/2026-09-26-c/`, gitignored)

| Gate | Result |
|---|---|
| `cargo xtask db test-it` (full) | 1,075 passed, 0 failed (`p7-db-test-it-final.log`); C prefixes: audit_replay 11 (10 in `audit_replay.rs`, 1 in the `audit_replay_shutdown` binary), audit_query_recovery 4, personnel_pagination 9, audit_frontend 6, vehicle_mutations 18, wiki_features 15, content_storage 15; kept: transactional_audit 1, audit_publication 2. The P3 phase gate ran 1,058 passed, 0 failed (`p3-db-test-it.log`) |
| `cargo test -p website-api --lib` | 515 passed, 395 before C; 55 are `wiki_markup_*` goldens (`p7-api-lib-final.log`) |
| Clippy `-D warnings` | website-api all targets clean (`p7-api-clippy.log`, re-run after G4); xtask clean (`p7-xtask-clippy.log`); ci `rust-clippy` exit 0 (`p7-ci-rust-clippy.log`) |
| rustfmt | ci `rust-fmt` diffs only in the other agent's `core/application_state.rs` and `community_content/models/generated/equipment_data_viewer/mod.rs` (`p7-ci-rust-fmt.log`); T's unformatted `tests/telemetry_url_guard.rs` formatted in C |
| Frontend lane (steps run one by one) | fmt diffs only in the other agent's `data_viewer` and equipment DTO parity files (`p7-frontend-fmt-2.log`); clippy wasm32 exit 0, no warning in a C file (`p7-frontend-clippy.log`); `cargo test -p website-frontend` 1,802 passed, 1 failed: `doc_audit`, all 100 findings in the other agent's untracked frontend apps/debug/data_viewer folder (93) and untracked dto/equipment_data_viewer folder (7) (`p7-frontend-test-2.log`); trunk release success (`p7-trunk-release.log`) |
| `mk leptos-gates` | exit 0, gate doctor OK, editor suite 21/21, DOM oracle 25/25 (`p7-leptos-gates-2.log`). The first run was 20/25 (`p7-leptos-gates.log`), diverging on exactly the five C routes; each was screenshot-audited and accepted with notes: personnel (pager), audit (live status badge; the fixture stream ends after `ready`, so it settles on Reconnecting), vehicles (administrator controls, compact header), wiki and wikislug (block renderer, revisions panel). The first wikislug accept was reverted because its screenshot showed leaked `view!` source in the revisions pager (an unbraced `disabled=page >= page_count`); G3 braced it and added the `view_attributes_*` guard. The 25/25 run predates G4's backend shutdown change; the oracle is fixture-driven and the frontend is unchanged since |
| `ci ci-local-schema` / codegen freshness | PASS: verify-codegen-fresh PASS, 230 `@contract` citations resolve (`p7-ci-local-schema.log`); a codegen re-run changes nothing |
| `verify route-tags` | 153 registered routes all documented; FAIL only on the other agent's 12 unwired `/api/v1/debug/equipment-data/*` tags (`p7-route-tags.log`) |
| `verify file-length` / `enfusion-comments` / no-select-star | 0 violations over 3,987 files / 0 findings / clean (`p7-ci-verify-coding-standards.log`) |
| Documentation gates | On the committed tree `ci verify-documentation` fails only on C's untracked new files (`p7-ci-verify-documentation.log`). With `--with-untracked`: readme-coverage finds only C's tracked-but-deleted files (cleared by `git rm` at commit) and the other agent's untracked equipment, `improved_layout` and `data_viewer` folders; link-check finds one line, the pre-existing T-1092 checkpoint link in `documentation_v2/mod/script_modularisation_progress_checkpoint.md`; markdown-placement OK (`p7-*-untracked.log`) |
| `ci ci-local` (steps run one by one) | verify-editorconfig FAIL, 3,205 findings, all in the other agent's untracked assets_v2/equipment folder; verify-no-python and verify-no-shell FAIL only on 9 tracked-but-deleted paths (C's 8 deletions and the other agent's `apps/mod/.mcp.json`), no banned path; verify-no-node, verify-ci-shell, verify-engine-layers, verify-staging-compose-paths, verify-mission-rest-size-limits, rust-build and wasm-ci PASS; rust-test-it is the final `db test-it` above; ci-local-leptos and ci-local-schema as above |
| Live walkthrough (browser pane, `rust-api-container` + `spa-gate-serve`, dev-login admin) | Personnel at `per_page=10` over 22 members (17 temporary users, removed afterwards): pages 1–3, Next disabled at the end, URL state survives a reload. Audit stream LIVE; vehicle create, PUT and soft DELETE rows arrived live; rows written while the API was down replayed exactly once after the restart. The vehicle form refused a `javascript:` image URL. `wiki-formatting-guide` renders H1–H6 with anchors, external links with `rel="noopener noreferrer nofollow"`, a lazy `no-referrer` image, an aligned table, disabled checklists, all seven callouts, code, quote and rule, and no script; two saves made revisions 2 and 3, a stale editor save got the 409 with a reload, and restoring revision 1 made revision 4. The walkthrough found that graceful shutdown never ended open event streams; G4 fixed it (streams close on SIGTERM and the process exits in about 3 ms; `audit_replay_shutdown` binary) |
| Perturbations (each red, then restored byte-identical) | personnel `total` capped to the item count (6 cases red); PATCH without `deny_unknown_fields` (parity case red); no table parsing (golden red); a publish failure skipping the read (recovery case red); the floor check skipped (4 reset cases red); the revision insert skipped (revision case red); `begin()` removed from the API shutdown (shutdown case red); the leaked-attribute defect restored (`view_attributes` guard red) |
| `verify api-readiness` (judge only, no `--execute`; `logs/H1-api-readiness.log` in the session scratchpad) | The register parses and validates (every implementation path exists), then the run stops at fingerprinting: `symlink fingerprint input: AGENTS.md`. `AGENTS.md` is a root fingerprint input and has been a tracked symlink to `CLAUDE.md` since 8db4105af (2026-09-26), so the command refuses any tree until that is resolved (open item). Readiness is not passing either way: V, S and B are open and no receipt is current |

Foreign files C touched, each only in its own separate hunk: `Cargo.lock` and
`apps/website/api_v2/Cargo.toml` (pulldown-cmark), `community_content/models/mod.rs`,
`community_content/routes.rs`, `community_content/services/mod.rs`, the other agent's untracked
`community_content/models/generated/mod.rs`, the frontend `core/api/dto/mod.rs` and
`tools_v2/xtask/src/commands/generate/schema_types.rs`.

## T in brief

- Every game-server route takes the server's machine credential; `ServiceAuth`, `SERVICE_TOKEN`
  and `X-Service-Token` are gone. `/metrics` and the detailed `/healthz` take the operator's
  `OBSERVABILITY_TOKEN`. `/api/v1/ingest/*` is on the global rate tier.
- Matches are registered per server before any report; results are numbered revisions with
  payload digests; event batches (seven kinds) are validated whole and counted once.
- The mod keeps outbound telemetry in a durable, bounded, checksummed queue under
  `$profile:TBD/Telemetry/`; heartbeats and server status carry its backlog, drops and oldest age.
- The dashboard answers the configured fleet (`servers.is_active`) with totals; the publisher and
  member reads skip inactive servers.
- Operator decisions of 2026-09-26 are recorded in `telemetry.md` (corrections merge per line with
  `removed_lines`; flat counter keys retired; T-1092's modularised mod layout used as is; events
  surface as a read route and DTO only).

## T verification (2026-09-26; logs in `target/api-progress-checkpoint/2026-09-26/`, gitignored)

| Gate | Result |
|---|---|
| `cargo xtask db test-it` (full) | 877 passed, 0 failed (`g-db-test-it.log`); T prefixes: match_identity 14, telemetry_revisions 7, telemetry_corrections 7, telemetry_atomicity 6, detailed_events 9, telemetry_queue 6, fleet_dashboard 6, statistics_recomputation 6 |
| `cargo test -p website-api --lib` | 395 passed |
| Clippy `-D warnings` | website-api all targets clean; xtask clean |
| rustfmt | clean on every file of this milestone (other agent's files not formatted) |
| Frontend lane | fmt step stops on the other agent's `data_viewer` files; clippy wasm32 exit 0; `cargo test -p website-frontend` 1,602 passed, 1 failed (`doc_audit`, only the other agent's `data_viewer` files); trunk release build passes |
| `mk leptos-gates` | editor suite 21/21; DOM oracle 25/25 after auditing and accepting `dashboard` (fleet list) and `servercontrol` (telemetry queue column, Inactive badge) |
| `ci ci-local-schema` / codegen freshness | PASS |
| `verify file-length` / `enfusion-comments` | 0 violations (3,829 files) / 0 findings |
| `verify route-tags` | 147 routes all tagged; FAIL only on 12 unwired equipment-viewer tags of the other agent |
| Documentation gates | markdown-placement OK; readme-coverage and link-check fail only on the other agent's equipment and improved_layout folders and the T-1092 checkpoint link |
| `mod compile` | clean |
| `mod world-boot` bare / `--compiled` | PASS / PASS (roll-call includes `MatchTelemetry=ok`; four-weapon equip) |
| `mod playtest --mission=6d0af8b0-… --timeout=420 --require-telemetry` | deployment CONFIRMED; heartbeat telemetry queue 0/512, dropped 0; pre-stop drain ran on timeout; credential revoked. No round went LIVE (0 players), so registration, events and results were not exercised in-engine |
| `cargo test -p xtask` | 1,043 passed, 4 failed: 3 in the other agent's files, 1 (`every_rust_file_named_in_prose_exists`) because this milestone's new tooling files are untracked until staged |
| `cargo test -p developer-tools` / `-p fleet-host-agent` | 265 passed, 4 ignored / 127 passed |
| `ci ci-local` | stops at step 1 (`verify-editorconfig`): 3,205 findings, all in the other agent's untracked assets_v2/equipment folder; the later steps were run individually above |

## Open items

- **B commit.** 55a6eab9f; T-940.10, the T-1177 remainder and T-1245 are shipped and stamped with
  it (bodies filled from the delivered evidence). B's NOTE findings are T-1252
  (API rebuild on a migration change), T-1253 (catalog version flag), T-1254 (signed-out
  Administration nav), T-1255 (`gate s-routes` workspace row), T-1257 (native frontend dead
  code); T-1256 files the under-barrel grenade launcher idea; the dead `null_tolerance_*` skip
  branches are added to T-1232. `ci rust-ci` passed on run 3.
- **V commit.** bd6ec3edf records V with its seven deletions; T-1041, T-1026, T-1012, T-1105,
  T-951, T-1055, T-944, T-949, T-1014, T-1088, T-1126, T-1173, T-1216 and T-1218 are shipped and
  stamped with it (`ticket check` OK). `verify api-readiness --execute` needs a quiet tree.
- **Perturbations not run.** The permission classifier refused these edits; the operator may
  run them: X1's race lock removal (the discriminating defect removes `lock_account` from
  `rotate_session`, V-F38), Q2's approved-artifact check removal, Q4's publication-order and
  capacity-check defects, R6's handler role-gate removal and R4's `missions_reviews` defects
  (V-F39, V-F47, V-F59, V-F83, V-F114).
- **Operator decisions.**
  - T-950 (the audit stream has no client): its scope is delivered by the audit page; ship or
    close it.
  - T-940.13 (combat, medical and vehicle telemetry events) is still `ready` from T.
  - T-1131 (whether Markdown edits invalidate readiness evidence), including V-F18: `CLAUDE.md`
    is not a fingerprint input, because `AGENTS.md` hashes as its link text.
  - T-1184 (SKIP_MIGRATE and RUST_LOG in the readiness digest), T-1109 (db test-it isolation for
    the rust-ci integration step), T-1163 (a server route for the mission export), T-1175 (the
    review workspace route's role tier), T-1209 (advertised server schema versions), T-1210 (the
    voice bridge radioClass values), T-1204 and T-1137 (the CI lanes for route-tags,
    no-select-star, doc-layout and the tool crates).
  - T-1241 (reports on a lapsed claim lease), T-1243 (closed request schemas whose handlers
    accept unknown fields), T-1244 (retiring the refusal-only `PATCH /admin/users/{discordId}`),
    T-1246 (the unread local `legacy/` export archive folder).
  - V-F115 (T-1250): the 19 Workbench-generated `Gameplay/Policy/Generated/` folders keep
    readme-coverage red; the fix renames the folder for the generator and refreshes the
    Workbench resource database.
- **Legacy audit rows.** Rows stored with a NULL `created_at` before C still read as
  `0001-01-01`; system rows appended since C stamp `now()`.
- **`view!` attributes.** A follow-up task was spawned to brace the unbraced `view!` comparisons
  outside C's pages (C's pages are braced and guarded by `view_attributes_*`).
- **Schema length.** `wiki-page.schema.json` is 764 lines. JSON schemas follow the precedent of
  `mission.schema.json` (1,713 lines) and are outside the `verify file-length` gate.
- In-engine exercise of match registration, events and results needs a LIVE round with a connected
  admin client (the two-client playtest runbook); the API side is covered by the integration
  suites.
- **Next:** commit B, then S with a quiet working tree for
  `verify api-readiness --execute`.

## Milestone V execution record

The milestone V execution record, launch amendments and findings pointer are archived verbatim in
[milestone_v_execution_record.md](/documentation_v2/archive/api_v2_completion/milestone_v_execution_record.md).

## Milestone C execution record

The milestone C execution record and launch amendments are archived verbatim in
[milestone_c_execution_record.md](/documentation_v2/archive/api_v2_completion/milestone_c_execution_record.md).

## Milestone B execution record

Plan: `/home/Samuel/.claude/plans/pasted-content-id-1072-resume-the-radiant-conway.md` (30 sub-agents in
8 waves plus operator checkpoint W; shared brief `b_brief.md`, facts `decisions.md` and prompts
`prompt_<agent>.md` in the session scratchpad). Operator decisions of 2026-09-28 are in the plan §3 and
the design note `game_ballistics.md`.

| Date | Phase handoff |
|---|---|
| 2026-09-28 | P0: shims, database, empty foreign snapshot (tree clean at 2365b1586), brief written; baselines (`target/api-progress-checkpoint/2026-09-28-b/p0-*.log`): map-engine 1,449 passed, 0 failed, 2 ignored; frontend 1,826 passed, 0 failed; `db test-it` taken from V's final run (1,302; only a documentation commit since). Wave 1 launched: B01, B02, B03, B04, B06, B15, B16. |
| 2026-09-28 | B01 done: `game_ballistics.md` (367 lines; link-check 20/20, markdown-placement and readme-coverage OK). Its NOTE findings became amendments to B09, B11 and B27. |

| 2026-09-28 | B04 done: ballistics-catalog, ballistics-calibration and fire-mission v2 schemas; `ballistics_validation` check (10 tests, NOT RUN until the fixture lands; two perturbations red); schema-codegen changes nothing; `schema citations` red on 4 dangling tags the API and frontend slices remove. Orchestrator decision: B07 waits for checkpoint W (the strict schema requires the oracle run), so B10 is not needed.

| 2026-09-28 | B14 launched early (depends only on B04).

| 2026-09-28 | B16 done: `format_grid`/`parse_grid` (6/8/10 figures, cell centre) and `overlay/fire_mission_marks.rs` on the existing marker, connection and zone lanes; 26 tests (11 + 15) twice green; four perturbations red; wasm32 clippy clean; native clippy red only on B06 in-flight test code (needless_range_loop) and the full crate on B15 in-flight `full_resolution` test. Stale overlay tests README entry → B27.

| 2026-09-28 | B06 done: `flight_model/` (RK4, Hermite crossing, trajectory) and `wind.rs`; 22 tests (15 + 7) twice green incl. the export spot check (45°, coef 1: 427.31 m, 9.417 s, 108.73 m); clippy native and wasm32 clean; eight perturbations red; `libm` optional under `scenario`.

| 2026-09-28 | B05 launched (B04 and B06 done).

| 2026-09-28 | B05 done: `catalog/` (weapon, shell, lookup) and `angular_units.rs`; 25 tests twice green (schema conformance via a `jsonschema` dev-dependency; the side-drag omission test destructures `FlightParameters`); clippy native and wasm32 clean; four perturbations red. B08 launched.

| 2026-09-28 | B14 done: migration 0060 (catalog table with immutability trigger, fire-mission input columns, `fire_mission_guns`, preserve-and-detach of dangling event ids, event FK ON DELETE SET NULL after the matches precedent) pinned; models; `game_ballistics_migration` 5 cases red-first then twice green, `db_migrate` and the pin suite green; two perturbation sets red; clippy clean. Save and list routes stay broken at runtime until B19 selects the new columns (B19 amended).

| 2026-09-28 | B02 done: tbd-export `BallisticsOracle` (edit-mode plugin for BallisticTable forward angles and altitude probes; play-mode component on the export game mode for `GetProjectileSimulationResult`, whose compiled signature adds `mustFallDown`, maximum time and horizontal distance and returns the end position, TOF by 16-step bisection; needs the engine projectile-debugging diagnostic; SHA-256 self-test; sidecars). `mod compile` and a probe compile 0 TBD warnings, enfusion-comments 0. Not run in Workbench (checkpoint W). The two `resourceDatabase.rdb` changes (11:52) come from the operator's running Workbench, not from B: added to the foreign snapshot, never staged by B.

| 2026-09-28 | B03 done: `apps/website/offline-service-worker` (lib: cache names, request classes, Range slicing, offline pack; wasm32 bin), 17-line loader, webmanifest, `index.html` worker link, crate-direction rule; 22 crate tests twice green; clippy host and wasm32 clean; trunk release passes; `repository_laws` 76; engine-layers PASS; perturbations red. Trunk writes unhashed worker files at dist root. Navigations network-first. A throwaway Node smoke of the built worker ran from the scratchpad (outside the repo). B18 launched.

| 2026-09-28 | B15 done: `BootstrapScope` (terrain-and-imagery boot skips world objects, forest, water and labels) with a full 2 m `FullResolutionDem` handle; `frontend/src/v2/core/map_view/` seam; the Mission Creator moved onto it (`canvas_mount.rs` 499 → 447; T-1045 lines fixed in `bridge/mod.rs`). map-engine clippy clean; tests: full_resolution 7 twice, map_view 17, editor 1,037, frontend full 1,843; trunk release ok; `gate editor-suite` 20/21 (the `hydrate` smoke needs the API; the orchestrator runs it at the wave gate). Three perturbations red. Frontend wasm32 clippy `--all-targets -D warnings` was already red before B (≈173 findings outside B; ci-local-leptos runs it without `-D warnings`), B15 fixed the 12 in its files. Orchestrator decision: the wave-1 gate runs together with the wave-2 gate, because B08 and B18 edit the same crates now.

| 2026-09-28 | Wave 1 complete (B01–B04, B06, B15, B16). B20 launched early (depends on B04 and B05 only). Waiting on checkpoint W for B07.

| 2026-09-28 | B08 done: `solver/` (golden-section max-range angle, Brent root capped at 60 iterations, typed refusals, per-ring rows, lowest-solving-ring recommendation), old solver and its tests deleted, API on a temporary `legacy_flat_solution` shim; 18 tests three times green; map-engine full 1,539 passed, 0 failed, 2 ignored; clippy native and wasm32 clean; six perturbation sets red. Its finding F3 is a defect (with wind set, the azimuth is geometric and the solution misses the target): orchestrator FIX via new agent B08b (wind-corrected aim, TimeToLive bracket), launched now; B11–B13 amended to use its fields. Stale mortar README and feature-doc links to the deleted file go to B24 and B27.

| 2026-09-28 | B20 done: `dto/fire_missions.rs` (shared `SavedFireMissionAnswer`, T-1245) and `dto/ballistics_catalogs.rs` with parity tests; `token_type` claimed as Bearer (T-1245); `dto` 161 passed, 6 failed: 3 goldens await re-capture (B21b amended with the event list golden) and 3 inline tests red on four `ChargeSolution` fields B08b added without the schema (B08b amended by message to add them). Its perturbation was refused by the permission classifier; the orchestrator runs it at the gates.

| 2026-09-28 | B23 launched early (depends on B20).

| 2026-09-28 | B18 done: `frontend/src/v2/core/offline/` (registration at boot, pack download on the first mortar visit, quota and persist, state on `<html>`); `map-tile-index.schema.json`; `cargo xtask map tile-index` (Everon: 5,461 tiles z0–6); API and Caddy serve `service_worker.js` no-cache; offline 19 + tile index 7 + router 2 tests twice green; four perturbations red. B22 launched.

| 2026-09-28 | B08b done: `solver/wind_corrected_aim.rs` (outer aim iteration, calm air bit-identical, TimeToLive bracket), four `ChargeSolution` fields added to the schema; solver 23 tests twice green (impact within 0.05 m of the target under crosswind, quartering, Δh); clippy native and wasm32 clean; three perturbations red. Three frontend DTO tests need the four keys in their hand-built rows (B21b amended). B11 and B12 launched.

| 2026-09-28 | B23 done: `pages/administration/ballistics_catalogs/` (upload form, version list, validation report) with route, nav and breadcrumb; 19 tests green; perturbations red. It added a multipart client verb `api_post_form_keeping_refusal` (`Body::Form`) in `core/api/client/requests.rs` outside its list; reviewed by the orchestrator and accepted (14 lines, no duplication of the refresh path). The admin nav entry changes the sidebar on admin oracle routes (B26 amended); doc-audit item in `core/offline/mod.rs:98` queued for the closing fix run; admin page feature doc → B27.

| 2026-09-28 | B12 done: `battery.rs`, `crest_clearance.rs`, `agreement_cases.rs` (splitmix64 lattice over the catalog); 27 tests green; map-engine full 1,564 passed, 2 ignored; clippy clean; four perturbations red. Its finding (no shared `FireMissionSolution` assembler) → new agent B12b (one assembler plus the mismatch comparison for API and page), launched after B11; B22 told by message, B19 and B25a amended.

| 2026-09-28 | B11 done: `dispersion.rs` (finite-difference PE ellipse, documented interpretation, InitSpeedVariation in m/s) and `fuze.rs` (burst-point time fuze with its own aim); 15 tests; map-engine 1,564 passed, 2 ignored; clippy clean; eight perturbations red. Orchestrator review: the prompt expected range PE to grow with range; on the high-angle branch only the speed share does (F1), so the ring-2 test asserts the speed share — a corrected expectation, recorded, not a weakening. F2 (the burst aim is missing from the wire `FuzeSetting`) → B12b amended to own that definition. B12b launched.

| 2026-09-28 | B12b done: `fire_mission.rs` (one assembler, `SOLVER_REVISION` game-ballistics-1) and `fire_mission_comparison.rs` (mismatch rule); `FuzeSetting.burst_aim` + `FuzeBurstAim` in the schema; 15 tests four times green; map-engine 1,579 passed, 2 ignored; clippy clean; two perturbations red. Frontend DTO `FuzeSetting` must accept `burst_aim` (B21b owns `dto/fire_missions.rs` now); the `fuze.rs` partial projection tag goes to the closing fix run.

| 2026-09-28 | Map-engine gate after B05–B16 and B08b/B11/B12/B12b (`g1-*.log`): `cargo test -p website-map-engine --all-features` 1,601 passed, 0 failed, 2 ignored (127 ballistics cases); clippy all-features all-targets `-D warnings` clean; clippy wasm32 default features `-D warnings` clean. Waiting on checkpoint W (B07 → B09 → B13, B17, B19) and B22.

| 2026-09-28 | B22 done: mortar page restructured (`catalog_source`, `inputs/`, `solve_bridge` mapping onto the shared `solve_fire_mission`), page public with only the save area behind sign-in; mortar tests 40/40 twice; perturbations red. Frontend full 1,913 passed, 7 failed, all in other agents' pending work (six `r_api::fire_missions` goldens → B21b; doc-audit item → closing fix). B24 launched (map mount with heights, crest profile, save button, public nav link).

| 2026-09-28 | Checkpoint W: operator restarted Workbench on tbd-export (the NET API cannot compile new plugin classes); the orchestrator drove the rest via enfusion-mcp: plugin run (operator typed the generation id in its dialog) → `forward_angles.json` complete (7 shells, no errors, sha256 9679868b…4d6b); export world played → `simulation.json` complete (4,185 samples, no shell errors, sha256 31d3b3c2…2e83; the availability probe passed without a diagnostic toggle); engine gravity `PhysicsWorld.GetGravity` = 9.8100004196167 (f32). Both sidecars match; copied to `assets_v2/scratch/ballistics_oracle/6A6F008DC5395616/` (gitignored); `active_generation.txt` removed; play stopped. B07 launched.

| 2026-09-28 | B24 stopped at its budget: map picker (markers, connections and zone meshes through the engine overlays), solution cards, saved fires split with the event picker moved in, offline status, public nav link, `allow(dead_code)` removed from `core/map_view`; old mortar files deleted (old tests kept in the scratchpad; removed cases listed with reasons). Frontend does not compile (two E0308: the page passes the engine `FireMissionSolution` where the DTO still has a copy). Orchestrator split: B20b (DTO re-export, legacy DTO removal, parity rows) now, then B24b (tests twice, perturbations, clippy, trunk). Foreign snapshot re-taken (the Workbench restart rewrote the two `.rdb` files again).

| 2026-09-28 | B20b done: `dto/fire_missions.rs` re-exports the engine solution types, legacy `FireSolution` DTO and its test deleted, parity rows carry the aim fields and `burst_aim`; the stored row keeps the const-false dispersion guard. Frontend compiles; `dto` 163 passed, 3 failed (the three goldens B21b re-captures); clippy wasm32 clean in its files. Its perturbation was refused by the permission classifier (orchestrator owes it); the engine-side const-false enforcement goes to the closing fix run. B24b launched.

| 2026-09-28 | B24b done: mortar suite 59/59 twice (one test input moved 1 km south because its target lay outside the 12,800 m Everon DEM; assertions unchanged, orchestrator reviewed); clippy wasm32 clean in mortar; trunk release ok; two perturbations red. Frontend full 1,934 passed, 4 failed (three fire-mission goldens and `SavedFire` column parity → B21b; doc audit `core/offline/mod.rs:98` → closing fix).

| 2026-09-28 | B07 done: `cargo xtask ballistics trim-export` (deterministic, three runs byte-identical), committed vanilla catalog (gravity 9.81 from the oracle; default-charge standard dispersion) and calibration bundle (31 native + 31 wind tables, 6,231 oracle samples) with four negative bundles; `schema validate` ballistics 4 PASS; 17 tests twice green; perturbations red. Deviation: the game tables use 12.5-mil and finer steps near vertical and the oracle sampled every 25 mils, so of 476 native rows 406 matched directly, 62 at lattice ends, 7 placed by interpolation, 1 left out. Operator decision: finer oracle re-run (1.5625-mil forward lattice) and a strict re-trim with no interpolation or omission: B02b (plugin) now, then the operator restarts Workbench, the orchestrator drives the run, then B07b. Non-charge tables dropped (only charge coefficients are fired in game). B09 launched on the current fixture (its code does not depend on the row evidence rules).

| 2026-09-28 | B02b done: oracle forward lattice 1600 → 800 mils in 1.5625-mil steps (whole sixteenths, exact decimals), revision `tbd-ballistics-oracle/2` (one-line constant bump in `TBD_BallisticsOracleRun.c`); mod compile and probe 0 TBD warnings; enfusion-comments 0. Estimate 52,839 forward samples, ≈13.9 MB, ≈8–10 s. Operator asked to restart Workbench for the second oracle run.

| 2026-09-28 | Oracle run 2 (operator restarted Workbench and typed the generation id; the orchestrator drove the rest via MCP): `forward_angles.json` revision 2 complete (13,888,200 bytes, 1.5625-mil lattice, sha256 cadf9f9b…33e4); `simulation.json` complete (4,185 samples, 0 shell errors, sha256 591d3147…d16d); sidecars match; revision-1 output kept as `.revision1`; play stopped; foreign `.rdb` snapshot re-taken. B07b launched.

| 2026-09-28 | B07b done: strict trim restored — every native row of the 31 tables matched (414 by a forward sample, 62 by the lattice-end rule the engine needs: it answers TOF −1 at both lattice ends of every table); nothing interpolated or omitted; calibration sha256 12be201b…d3d0 (catalog unchanged 24a68cc5…fbde); three trims byte-identical; 18 tests twice; schema validate ballistics PASS ×4; perturbation red. B09 told by message (fixture regenerated; its clippy finding).

| 2026-09-28 | B09 done with red: `calibration/` (native, wind, oracle, provenance, report, pure-Rust SHA-256) and 27 tests; 20 green (provenance, four negatives, the red-case checks, column_1 = range·tan θ), the seven per-shell committed-bundle tests red — no tolerance loosened. Residuals: R1 5,262 of 15,841 forward samples (the engine forward lookup is linear interpolation of its sparse table between rows, not flight); R2 30 native rows and 258 wind-row ranges at 40–48° (±1 mil window under 0.1 m wide, model max range −0.07…+2.9 m off); R3 84 simulation samples overshoot range ≈0.11 % (limit ≈0.1 %), TOF within 0.05 s; R4 75 near-vertical wind cases at mm scale. Findings: `m_aValues` = [crosswind deflection mrad, head/tail range change m, angle of fall deg]; SideAirDragScale not applied (drift ratio 0.998–1.002 without, 7.95–9.93 with). Orchestrator: the g = 9.807 preference against the engine's true 9.81 points at a different engine integrator; B09b launched to identify it from the oracle and make the flight model engine-faithful. Criterion questions (R1, R2, R4) go to the operator after B09b.

| 2026-09-28 | B09b done: engine integrator identified from the 4,185 oracle simulations — fixed 1/30 s step, gravity first, linear air-relative drag, position by mean velocity, linear chord crossing (f32: max 0.0078 m, RMS 0.0013; f64: 0.016 m; RK4 reference 2.87 m); flight model rebuilt on it (f64); native rows within 0.011 m / 0.0007 s; simulation failures 84 → 0; flight tests 17 green; RK4-specific expectations moved to the scheme's chord-sag bound (listed, orchestrator reviewed); perturbation (RK4 back) red. Remaining: R1 forward samples between rows, one wind row at 42° (0.003 m outside the window), 54 crosswind rows (judge decoding bug), 3 solver vacuum tests assuming RK4 exactness. Operator decisions: R1 forward samples are row evidence only; the model moves to f32 state like the engine (no tolerance floor). B09c launched.

| 2026-09-28 | B09c done: flight in explicit f32 with the engine constants (simulation residual max 0.0078 m, native rows 0.0057 m / 0.0008 s); forward samples judged only at native rows (414 judged, 15,427 counted as engine table interpolation); crosswind judge compares the decoded angle within 1 mil; solver vacuum tests bounded by chord sag plus one f32 rounding per step (still discriminating); Brent converges on all 21 f32 brackets; all seven per-shell calibration tests green with unchanged tolerances; three perturbations red. One red: dispersion calm-air symmetry at f64-level tolerance under f32 noise. Orchestrator decision (delegated reasoning, no tolerance change): dispersion is a documented interpretation and a derivative, so it runs on the f64 copy of the same engine step with h restored to 1e-5 rad; the upload report gains the unjudged forward-sample count. B09d launched.

| 2026-09-28 | B13 and B17 launched (calibration green).

| 2026-09-28 | B09d done: dispersion runs on the f64 copy of the engine step (h 1e-5 restored; a new test bounds f64-vs-f32 impact separation at 0.02 m and proves the paths differ), `forward_samples_not_judged` serialised in the upload report and required by the schema; ballistics 167 tests twice green; clippy native and wasm32 clean; two perturbations red. It added a public `fly_to_height_double_precision` wrapper in `flight_model` (outside its list; reviewed, accepted). B17 told by message to carry the new field; frontend DTO field → B21b.

| 2026-09-28 | B17 done: catalog services and handlers (admin multipart upload → engine calibration → immutable insert + audit in one transaction; public list and detail with ETag/304 and immutable caching); `game_ballistics_catalog_upload` 8 twice green (vanilla 201 with 15,427 unjudged forward samples, four negatives 422 pinned to their case ids, 409, 403/401, 405, 400/415, 413 at limit+1) + 5 unit; route-tags 168/168; four perturbations red. Calibration bundle is 6.4 MB: part caps 16 MiB / 1 MiB, route 17 MiB + 64 KiB. api clippy all-targets blocked only by `fire_mission_solution.rs:465` including the deleted mortar `grid.rs` → B19. B19 launched.

| 2026-09-28 | B13 done with red: 25 sweep tests (symmetry 4, bounded failure 11 over 10×1,000 seeded draws, oracle elevation and wind 8, end to end 2); 18 green, 7 oracle tests red on 314 samples — a solver defect (F1: the aim loop's first iteration aims at the target ignoring wind and a refusal there ends the solve, so reachable crosswind targets near 45°/85° are refused) → new agent B08c. Orchestrator review of deviations: symmetry holds to 0.01 mil / 1e-3 s instead of 1e-9 because rotation changes the f32 rounding of the engine-faithful flight (operator chose f32 state; exact quantities stay 1e-9); oracle inversions at exactly 45°/85° allow the 1-mil tolerance at the limits; 147 low-branch oracle samples are checked to resolve to the high-angle answer. Accepted and recorded. Three perturbation sets red.

| 2026-09-28 | B08c done: after a range-bound refusal the aim loop re-aims from the refused bracket-end flight's drift (refusal kept only if the aim point stops moving or the last iteration is refused; solved and calm paths bit-identical); B13's 7 oracle tests green unchanged (0 of 314 fail); ballistics 196 passed twice; clippy native and wasm32 clean; perturbation (early return) red.

| 2026-09-28 | B25a and B25b launched in parallel with B19 (end-to-end runs wait for B21b's catalog goldens).

| 2026-09-28 | B19 done: `handlers/fire_missions/{save,list}`, `services/fire_mission_resolve.rs` (pinned catalog, shared assembler in `spawn_blocking`, `compare_solutions`; 422 `solution_mismatch`/`fire_mission_refused`/`no_firing_solution`) and `fire_mission_store.rs` (one transaction with guns, FK 404); solve route and `legacy_flat_solution` removed; `game_ballistics_fire_missions` 9/9 on three runs; `fire_mission_solution` 4/4 (solve cases removed, pin re-pointed line-exact to `restore.rs::parse_legacy_grid`); api clippy all-targets clean; route-tags 167/167; schema citations 482/482; perturbation (tolerances 0) 6 red. It ran `git rm --cached` once and reset the entry itself; the orchestrator verified the index is empty. Findings → closing fix run: `serde_json` float_roundtrip, the dropped pin doc assertion, a 12-gun battery cap (orchestrator decision); `fixture_router` solve pin → B21b. B21a and B21b launched.

| 2026-09-28 | B21a done: solve block removed from `admin_approvals_cms_field_tools.rs` (980 → 959); null tolerance extended with a catalog-model fire mission, a gun and both catalog GET routes (`null_tolerance_reads` red-first, then 3/3 twice; select scan 1/1); `admin_approvals…` 5/5, `fire_mission_solution` 4/4, `game_ballistics_catalog_upload` 8/8; seed §15 comment only; perturbation red. Finding: `select_literals` cannot see `concat!`-built SELECTs, so the select scan is fail-open on `fire_mission_store.rs` → closing fix run (FIX class).

| 2026-09-28 | B25b stopped past its budget (≈305k; orchestrator asked it to wrap up): `gate mortar-offline` (plan, page driver, expected native solution, map-pixel check) with a corpus-backed `/api/` route in the gate server; `Network.setBypassServiceWorker` in doctor, editor-suite/smokes/render-check and v-suite; the offline crate in all four `wasm-ci` steps and the CI task row; `mk mortar-offline-gate` recipe; 17 developer-tools + 3 xtask tests; clippy clean; perturbations red. End to end on a scratch corpus from the committed catalog: every step through `mission_entered` ok, `solution_matches_native` red (page elevation 1179.8 vs native 1179.9 mil, apex 1 m lower; inputs identical: grids at cell centres, manual heights, same wind). Orchestrator hypothesis: display truncation on the page (both differences one display unit lower); B25a's raw-bits bench decides; follow-up B25c after B25a reports. Goldens still await B21b.

| 2026-09-28 | B21b done: goldens captured from the real `api` built from the tree on a fresh seeded database (vanilla pair uploaded through the route: 201, 7,865 cases, 15,427 unjudged forward samples; save captured with the server's own solution); new `operations_ballistics` route-acceptance part (8 twice), `contract_parity_goldens` 6/6 twice, coverage 14/14, reservations part 8/8, `json_rejection_envelopes` 34/34, frontend dto + admin 185/185, developer-tools fixtures 26/26; five perturbations red. FIX inside its files: `fire_mission_saved` asserted the geometric azimuth; the API stores the wind-corrected aim. Link-check on the evidence folder fails only on a backticked path to the untracked offline crate (clears at commit). G1 (closing fixes batch 1) launched.

| 2026-09-28 | B25a done: URL-only bench `/debug/ballistics-agreement` and `gate ballistics-agreement` (served golden checked against the committed catalog; native solve through the shared assembler); `cargo xtask mk ballistics-wasm-agreement` against the real goldens PASS 32/32, bit-identical 32/32 (`B25a-recipe.log`); developer-tools 306 passed, 4 ignored; 20 + 13 new tests twice green; perturbation (×(1+1e-3)) 29 red, 0 bit-identical. The shipped WASM equals native bit for bit, so B25b's 0.1-mil page difference is on the page side (display or input), not the model. Case mapping duplicated in bench and gate → closing fixes batch 2.

| 2026-09-28 | B27 launched (docs; parallel with G1).

| 2026-09-28 | G1 done (9 of 10 items): offline doc + no allow; fuze tag marked partial; engine dispersion refuses `verified_in_engine: true`; api `serde_json` float_roundtrip; the pin's under-permissive assertion restored; 12-gun battery cap (schema, handler 400, page); select scan reads `concat!` + `name!()` macros and fails on an unreadable part (it found 7 more legacy Option columns); golden recipe gains the upload step. map-engine ballistics 197; six API binaries twice green; frontend mortar/offline/doc_audit 90; clippy clean; perturbations red. Not done: the 34 native dead-code warnings in mortar — the native frontend build already had 124 warnings before B (nothing mounts the app natively): NOTE for a ticket (B28), not a B defect. NOTE: `null_tolerance_select_scan` skips without `TEST_DATABASE_URL` (the readiness judge refuses skip output; ticket). B25c and G2 launched.

| 2026-09-28 | B27 done: mortar page and admin catalogs feature docs, map-engine ballistics docs (`game_ballistics_engine.md`), runbooks (`ballistics_oracle_run.md`, `offline_mortar_page.md`), glossary (four terms), `api_overview.md`, EVIDENCE README count and row, design note at 448 lines with the engine scheme and decisions, middleware body-limit line, code README Contents fixes. It left CLAUDE.md to the orchestrator with a 14-line atlas patch; reviewed and applied by the orchestrator (plan-approved atlas update). Documentation gates with untracked files: markdown-placement OK; readme-coverage 22 (19 are the pre-existing `Policy/Generated/` folders, T-1250; 3 clear at commit); link-check 1, a stale pre-B citation of the removed `doc_audit/allowlist.rs` → closing fixes batch 2.

| 2026-09-28 | G2 done: case→inputs mapping, lead summary and bit walk live once in map-engine `agreement_cases.rs` (bench and gate import them; mirror test retired as one source remains); stale allowlist citation rewritten; two Related links added; map-engine ballistics 201; developer-tools and frontend bench tests twice green; `mk ballistics-wasm-agreement` PASS 32/32 bit-identical 32/32; link-check with untracked OK; perturbation red.

| 2026-09-28 | B25c done: the 0.1-mil page difference came from a stale `dist` (the page predated the B08c solver edits), not from truncation; a fresh recipe passed unchanged. Shared `solution_wording.rs` in map-engine (round to nearest, ties to even) used by the page and the gate; the gate names the failing step; map-engine 1,660 passed, 2 ignored; frontend mortar 61 twice; developer-tools mortar_offline 21; `cargo xtask mk mortar-offline-gate` PASS in 105 s (trunk 80 s); perturbation (page truncation) red in two frontend tests and in `mortar_offline_solution_matches_native`. B26 launched.

| 2026-09-28 | Final gate run 1 (`f-db-test-it.log`, after pruning rebuildable caches: /home 84 → 142 GB free): 1,338 passed, 1 failed, 0 ignored over 152 runner sections; `game_ballistics*` 29 cases. The failure: `POST__fire-missions.json` dispersion deflection PE differs by one ulp from the live answer — the golden predates G1's serde_json `float_roundtrip`; G3 re-captures it through the recipe.

| 2026-09-28 | G3 done: `POST__fire-missions.json` re-captured through the recipe (one byte: …496 → …497); all 90 index rows replayed with matching statuses; no other golden affected; `contract_parity_goldens` 6/6 twice; frontend `r_api` 137. Static gates (`f-*.log`): route-tags 167/167; file-length 0 violations over 4,306 files; enfusion-comments 0; engine-layers PASS; `ci ci-local-schema` PASS (482 citations); mod compile 0 TBD warnings. Map-engine: 1,682 passed, 0 failed, 2 ignored; clippy `-D warnings` clean all-features, each of the eight feature sets (lib), wasm32 default and all-features.

| 2026-09-28 | B26 done: `v-suite verify` 26/26 routes match the frozen oracle; `mortar` public, new `ballistics_catalogs` route and golden; 24 routes accepted after audit — for all 22 sidebar routes the new DOM minus the new nav link equals the old golden exactly, PNG changes are the sidebar only; the mortar golden shows the offline pack state without a service worker (named in its note). FIX inside its scope: the fixture router's canned refresh lacked `token_type` (B20 refuses it; every seeded capture signed out) + 1 test red-first. The editor smokes' canned refreshes lack it too → G4. NOTE tickets: signed-out Administration nav, `gate s-routes` missing a pre-B route row.

| 2026-09-28 | Final gates, backend and frontend (`f-*.log`): full `db test-it` run 2 1,339 passed, 0 failed, 0 ignored (game_ballistics 29, route_acceptance 73, contract_parity 86, controlled_races 9, failure_injection 27, engineering_laws 10; no skip lines); `website-api --lib` 543; api clippy `-D warnings` all targets and `--lib --bins` clean; frontend fmt clean, `cargo test -p website-frontend` 1,952 passed, 0 failed, wasm32 clippy no warning in a B line; xtask 1,058 passed, 1 red (`every_rust_file_named_in_prose_exists`: three new files still untracked, clears at commit).

| 2026-09-28 | G4 done: one `gate_refresh_answer` constructor (complete Bearer pair) used by the DOM oracle and every editor smoke; developer-tools 309 passed, 4 ignored; clippy clean; editor smokes 20/21 without the API (hydrate needs it); perturbation red in its test and B26's.

| 2026-09-28 | Final browser gates (`f-leptos-gates.log`, API on :8080, dev DB migrated to 0060): `mk leptos-gates` exit 0 — gate doctor OK, 21 editor smokes pass (hydrate included), DOM oracle 26/26. Live walkthrough started: vanilla catalog uploaded as admin through the API (201, 7,865 cases, 0 failures); the admin page lists version 1. The local `spa-gate-serve` launch config (untracked) mounted `--map-assets assets_v2/terrains/everon` instead of the terrains root; fixed. Walkthrough finding: the browser pane cannot reach `fonts.googleapis.com`, and one failed optional font request marks the whole offline pack `failed` → G5 (essential vs optional pack entries).

| 2026-09-28 | Live walkthrough, online (browser pane, gate serve + API; the pane has no WebGL2 so the map shows its fallback and grids are typed, heights manual — the map itself is proven by the headless gates): admin page lists the uploaded version; a 3-gun M252/M853A1 battery with wind 3.5 m/s from 250° solves on the page (per-gun charge, elevation, wind-corrected aim, deflection and range corrections, dispersion labelled as interpretation); a single-gun M821 mission (1179.9 mil, as the native gate) saved to a new upcoming event → `POST /api/v1/fire-missions` 201 (server re-solve agreed) and listed. Finding: the illumination fuze only tries the ground-recommended charge (burst 300 m refused although charges 3–4 reach it) → G6.

| 2026-09-28 | G5 done: offline pack splits essential and optional (cross-origin icon font) entries; optional-only failure ends `ready` with `data-offline-optional="missing"` and a visible notice (progress 99, documented); essential failure stays `failed`; frontend offline + mortar 86 twice; developer-tools mortar_offline 23; `mk mortar-offline-gate` PASS; perturbation red. Two README lines queued.

| 2026-09-28 | Live walkthrough, offline (API stopped, SPA still served by the gate proxy): after a `ready` pack (optional font missing, service worker controlling, cross-origin isolated), the reload shows "catalogs could not be loaded (502) … no offline copy" and the pack flips to `failed`: behind a proxy a stopped API answers 502, and neither the service worker (network-first falls back only on network errors) nor the page falls back to the cached catalogs; the gate missed it because it stops the whole listener. Defect → G7 (5xx fallback in the service worker and page, no downgrade of a complete pack, a 502 step in the gate).

| 2026-09-28 | G6 done: `solve_time_fuze_over_charges` (lowest ring whose burst aim solves inside the fuze window; refusals `OutsideFuzeWindow` / `NoBurstCrossing`), operator charge still pins the fuze, `SOLVER_REVISION` game-ballistics-2; the fuze card names the real cause; ballistics 212 twice; clippy clean; perturbation red. Red: `game_ballistics_fire_missions` hard-codes the old revision; the save goldens carry it → G8 (tests on the constant, golden re-capture, a precise fuze refusal cause in the contract, fuze docs, two README lines).

| 2026-09-28 | G8 done: API tests assert the exported `SOLVER_REVISION`; save goldens re-captured through the recipe on fresh databases (only `solver_revision` 1 → 2; 91 rows replayed); `FuzeRefusal` carries the precise causes (above apex, beyond range, inside minimum range, outside fuze window, …) in the engine, contract and fuze card; fuze docs, glossary entry and the core README offline line updated; ballistics 213, fire missions 10, goldens 6, frontend r_api + mortar 206 (each twice); schema citations 483; three perturbations red. It stopped its probe API with a host `pkill` on the api binary name (no other API was running).

| 2026-09-28 | G7 done: service-worker `network_fallback` (network error or any 5xx → saved copy, 4xx passed through, `x-served-from-offline-cache` marker); page `saved_copies` with dated copies; a failed refresh keeps a complete pack `ready` (`data-offline-refresh=kept-saved-copy`); one shared key function for pack and page; gate `mortar-offline` gains an API-down-behind-proxy (502) visit — 15 cases PASS; tests twice green; perturbation red in the SW test and the gate. Walkthrough re-run: online visit filled the pack (`ready`, optional font missing, refreshed); API stopped (proxy answers 502) → reload shows "Offline copy from 28 Sep 2026, 15:25 UTC", pack stays `ready`, M252/M821 with wind solves offline to 1179.9 mil (identical to online). Remaining UX gap: offline, the save area shows the sign-in prompt instead of "saving needs a connection"; G7 dropped the 429 retry on catalog reads → G9.

| 2026-09-28 | G9 done: offline save area says saving needs a connection (no sign-in prompt offline); one shared 429 retry helper for `public_get` and the saved-copy reads; frontend 109 twice; `mk mortar-offline-gate` PASS (15 cases); two perturbations red; `page.rs`, `client/public_reads.rs`, `client/mod.rs` edited outside its list (reviewed, accepted). Re-gates: map-engine 1,687 passed, 0 failed, 2 ignored, clippy native all-features, per feature set and wasm32 clean; full `db test-it` 1,339 passed, 0 failed, 0 ignored (game_ballistics 29, route_acceptance 73, contract_parity 86). Orchestrator perturbations (`p-*.log`, each restored sha256-equal): drag term zeroed → 11 calibration red; Δh ignored → 17 red; fixture row +5 m (m821 2.541 row 4) → 4 red naming the row; wind sign flipped → 29 red; comparison tolerances 0 → the skewed-save case red; token_type any scheme → `session_refresh_refuses_a_token_type_other_than_bearer` red; bench TOF ×1.01 → WASM gate FAIL 0/32; frontend dispersion guard disabled → nothing red (redundant since the engine type refuses the claim) → G10 removes it.

| 2026-09-28 | Final sweep (`z-*.log`): G10 removed the redundant frontend guard (test proven by the engine guard); G11 documented two items and formatted one test. Frontend fmt clean, 1,982 passed, 0 failed; developer-tools 313 passed, 4 ignored, clippy clean; offline crate 28, clippy host + wasm32 clean; xtask 1,058 passed, 1 red (prose rule, untracked files); `mk leptos-gates` doctor OK, 21 smokes, 26/26; `mk ballistics-wasm-agreement` PASS 32/32, bit-identical 32/32; `mk mortar-offline-gate` 15 cases PASS; documentation gates: link-check PASS, markdown-placement red only on this checkpoint (527 lines → B28 archives the V record), readme-coverage 19 pre-existing generated folders + 7 entries of the 11 deletions; `ci ci-local` stops at verify-no-python on the 11 unstaged deletions (as in V); run one by one: editorconfig, no-node, ci-shell, engine-layers, coding-standards, staging-compose-paths, mission-rest-size-limits, ci-schema-parity PASS, no-shell red only on the deletions; `ci rust-ci` running. B28 launched.

| 2026-09-28 | `ci` steps, final: `rust-ci` run 1 and 2 red at rust-fmt (two B files, then one import order in `xtask …/recipes.rs`), formatted by the orchestrator; run 3 (`z-ci-rust-ci-3.log`) PASS — rust-fmt, rust-clippy, rust-build, wasm-ci (incl. the offline crate) and rust-test-it, 3,098 passed, 0 failed, 2 ignored (the two pre-existing ignored map-engine tests, run unfiltered by wasm-ci); `ci ci-local-schema` PASS (483 citations); `mk ci-local-leptos` exit 0, 1,982 passed.
| 2026-09-28 | B28 done: register — seven B requirements (real implementation paths, assumptions: dispersion not verified in-engine, offline gate needs the local tile pyramid, tile index and tbd-sat, fixtures pinned to game build 1.8.0.13), five map-engine checks on one command (51 + 34 + 81 + 29 + 18 = 213, each pattern verified against `f2-map-engine-test.log`), the two gate checks (32, 15), `verification_game_ballistics` 29; backend_regression 1,339, frontend_quality 1,982, route_acceptance 73, contract_parity 86, browser_acceptance 26 with the 26/26 marker; judge-only `verify api-readiness` validates the register (FAIL without receipts). V execution record archived; `remaining_milestones.md` §B implemented; T-940.10 re-anchored with evidence, T-1177 remainder and T-1245 evidenced (ship after the stamp); T-1252 to T-1257 filed, T-1232 extended; `ticket check` OK; `game_ballistics.md` register table, fuze search and `SOLVER_REVISION` updated; documentation gates with untracked files: link-check and markdown-placement OK, readme-coverage 19 generated folders + 7 entries of the deletions. |

| 2026-09-28 | Committed as 55a6eab9f (557 files; the two Workbench `.rdb` files stay unstaged); T-940.10 shipped and stamped; T-1177 and T-1245 bodies filled from the delivered evidence, shipped and stamped; `ticket check` OK.

### Milestone B launch amendments

| Agent | Addition to the pre-written prompt |
|---|---|
| brief (all) | Path shorthands and the P0 baselines added at the top of the brief. |
| B09 | Wind-table rows judged by the range criterion (decision 3); native column 1 is B09's to decode against the oracle forward samples (design note open question 1). |
| B11 | Deflection spread maps to azimuth as δ/cos θ; the InitSpeedVariation working unit is m/s (design note open question 4). |
| B27 | Fix the EVIDENCE README count and domain table; restore the full `cargo xtask` citations and paths in the design note once they exist. |
| B07 | Launched only after checkpoint W; `--oracle` required for the committed fixture; schema validate must PASS the ballistics section. |
| B14, B19, B20 | Remove the dangling `@contract` tags to the retired FireSolveRequest/FireSolution in their files; B04's schema choices listed. |
| B27 | (add) overlay tests README Contents fix (B16 F3). |
| B05 | Owns the test that `side_air_drag_scale` does not enter `FlightParameters` (B06 F1). |
| B19 | (add) select the 14 new columns and load `guns`; `fire_mission_solution.rs` must pass; migration re-embed note. |
| B28 | File NOTE tickets: no API build.rs rerun for migrations (B14 F2) and every B NOTE finding. |
| B18 | Trunk worker naming, `BuildId::script_url()`, network-first navigations; a `map-tile-index.schema.json` contract for `MapTileIndex` with `@contract` tags. |
| B24 | Use the B15 seam; remove every `#[allow(dead_code)]` B15 placed on picker-only modules. |
| B08b | New agent (orchestrator FIX of B08 F3): wind-corrected aim and the TimeToLive bracket. |
| B11, B12, B13 | Use the solver's aim azimuth and drift correction (B08b). |
| B19 | (add) fix the `fire_missions.rs:38` doc link; delete `legacy_flat_solution.rs`. |
| B08b | (message) Owns the four new `ChargeSolution` fields in `fire-mission.schema.json`. |
| B21b | (add) re-capture the event fire-mission list golden. |
| B24 | (add) delete the legacy frontend `FireSolution` DTO and its golden test. |
| B22, B24, B25b | Offline state on `<html>`, `offline_status()`, the tile-index producer, aim fields (B08b). |
| B21b | (add) the four `ChargeSolution` keys in `r_api_fire_missions.rs` hand-built rows. |
| B26 | Accept a divergent route only when its whole diff is the new nav entry (or the mortar/admin pages themselves), each noted. |
| B27 | (add) admin ballistics catalogs page feature doc. |
| B12b | New agent: shared `fire_mission.rs` assembler and `compare_solutions`. |
| B19, B25a | Use the B12b assembler and comparison; `agreement_cases` takes the catalog. |
| B12b | (add) owns `FuzeSetting` in the fire-mission schema: burst-point aim. |
| B21b | (add) owns `dto/fire_missions.rs` for `burst_aim`; prefer re-exporting the map-engine types. |
| B24 | (add) heights mount, crest profile, event picker move, save button, public nav link (B22 F3, F4). |
| B07 | (add) oracle output paths, sha256 values and engine gravity. |
| B20b | New agent: frontend fire-mission DTOs onto the engine types (taken from B21b). |
| B24b | New agent: finish B24 (compile, tests twice, perturbations, clippy, trunk). |
| B21b | (add) DTO items moved to B20b; keeps golden re-captures and the solve golden deletion. |
| B21b | (add) `SavedFire` mirrors every new `FireMission` column (B24b F1). |
| B02b | New agent: oracle forward lattice 1.5625 mils, revision 2 (operator decision). |
| B07b | New agent: strict re-trim on the fine lattice; relaxed evidence rules removed. |
| B09b | New agent: identify the engine integrator from the oracle simulation samples; engine-faithful flight model; no tolerance changes. |
| B09c | New agent: f32 state, forward samples as row evidence only, wind judge decoding, solver vacuum bounds; calibration green with unchanged tolerances. |
| B27 | (add) identified engine scheme and the R1 and f32 decisions in the design note and catalog README. |
| B09d | New agent: dispersion on the f64 step (h 1e-5 restored), f32/f64 agreement test, `forward_samples_not_judged` in the upload report. |
| B13, B17 | Engine scheme and green calibration facts; B17 body limit from measured bundle sizes. |
| B21b | (add) `forward_samples_not_judged` in the frontend `CatalogUploadReport`. |
| B19 | (add) re-point the fire_mission_solution shipped-source pin to `restore.rs::parse_legacy_grid`; use `catalog_store::load_catalog`. |
| B27 | (add) middleware README body-limit row for the catalog upload. |
| B08c | New agent: wind-corrected aim continues past a first-step refusal (B13 F1). |
| B25a, B25b | Parallel launch; shared recipe/gate registration files; wait for B21b goldens for the end-to-end runs. |
| B21b | (add) re-point the `fixture_router` test off the solve golden. |
| G1 | Closing fixes batch 1: the items queued in `closing_fixes.md` (listed in its prompt). |
| B25c | New agent: page rounding fix and shared formatting in the offline gate; finish B25b end to end. |
| G2 | Closing fixes batch 2: one shared agreement-case mapping. |
| G2 | (add) the stale allowlist citation and two Related links (B27 F2, F3). |
| B26 | Fresh dist first; expected divergences and screenshot audit per route. |
| G3 | New agent: re-capture the fire-mission save golden after float_roundtrip. |
| G4 | New agent: every canned refresh answer in the browser gates carries `token_type: Bearer` (one shared constructor). |
| G5 | New agent: offline pack essential vs optional entries; optional font failure keeps `ready` with a visible flag. |
| G6 | New agent: fuze burst aim searched over every ring (walkthrough finding). |
| G7 | New agent: offline behind a proxy — 5xx fallback, pack not downgraded, 502 gate step (walkthrough finding). |
| G8 | New agent: revision constant in tests, golden re-capture, fuze refusal cause in the contract, fuze docs. |
| G9 | New agent: offline save-area wording; restore the 429 retry on catalog reads. |
| G10 | New agent: remove the redundant frontend dispersion guard; prove the engine guard carries the test. |
