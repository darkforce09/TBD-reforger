**Status:** live

# API v2 verification checkpoint — 2026-09-28

Milestones E, F, M, T, C and V are implemented and verified; overall readiness is **not
passing**, because staging (S) has not started, game ballistics (B) is a later phase (see
`remaining_milestones.md`), and no receipt exists: `cargo xtask verify api-readiness --execute`
was not run (it needs a quiet working tree). T is committed on main as 0ef292758 and C as
b49fb86c1; V is committed as bd6ec3edf (2026-09-28), with its fourteen tickets shipped and
stamped.

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
| B | Later separate phase by operator decision (2026-09-23). |

Migrations 0057–0059 are pinned; the next migration is 0060. Versions 0022–0024 stay retired.

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
  - T-1177 remainder: fire missions skip viewer access, and `fire_missions.event_id` has no
    foreign key (a migration and a dangling-row policy; V-F98).
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
- **Next:** S, with a quiet working tree for
  `verify api-readiness --execute`; game ballistics (B) follows in its own later phase by
  operator decision.

## Milestone V execution record

Plan: `/home/Samuel/.claude/plans/pasted-content-id-1946-resume-the-joyful-meadow.md` (roster of 32
sub-agents with pre-written prompts; shared brief in the session scratchpad `v_brief.md`). At the
start of V the working tree was clean: the other agent committed its equipment work as a967a75bb,
3be13084c and 9a8d56c59 (with 9574714ee), so no uncommitted foreign files exist; the foreign
snapshot is re-taken at each wave boundary.

| Date | Phase handoff |
|---|---|
| 2026-09-28 | P0 started: shims, database, empty foreign snapshot, brief written; baselines running (full `db test-it`, frontend tests). |
| 2026-09-28 | P0 baselines (`target/api-progress-checkpoint/2026-09-28-v/p0-*.log`): `db test-it` 1,075 passed, 0 failed (lib 515; build 86 s, tests 153 s over 115 binaries); every identity_* and administration_* check at its minimum with every named case; V checks as planned (five at 0, property 19/25, 7 property records missing). Frontend 1,802 passed, 1 failed: `doc_audit`, 100 findings, all in the other agent's committed 3be13084c (`apps/website/frontend/src/v2/apps/debug/data_viewer` 93, `apps/website/frontend/src/v2/core/api/dto/equipment_data_viewer` 7). `xtask api_readiness` 56 passed. |
| 2026-09-28 | Wave 1 launched: T0 (triage, read-only), A1 (design note), A2 (failpoints core), A3 (response contracts; inventory first), D1 (golden reproducibility). Prompts: plan §<agent>, stored as `prompt_<agent>.md` in the session scratchpad. |
| 2026-09-28 | Wave 1 done. T0: 57 triaged rows (FIX 13, CLOSE 9, NOTE 35; session scratchpad `triage.md`). A1: `verification_completeness.md` (355 lines, documentation gates pass). A2: `core/failpoints/` with `failpoints` feature and self dev-dependency; release `api` and rlib carry 0 catalogue names, test builds all 18; lib failpoints 16/16; clippy both configurations clean. A3: 169-row route→contract inventory; 19 new response schemas; 123 `@contract` tags in 53 files (T-1026 done); `ci-local-schema` PASS (353 citations). D1: all 53 goldens reproduce from `registry_dev.sql` + `content_golden.sql` (three fresh captures 53/53); 18 goldens changed, 3 `*.request.json` added; T-949 fixed red-first. |
| 2026-09-28 | Operator decisions on the triage: V fixes the other agent's committed red items (F-1 `doc_audit` 100 findings → L1; F-2 route-tags on the nested equipment router → K1; F-9 module header → L1); the `/api/v1/debug/equipment-data/*` routes become development-only like `/auth/dev-login` (F-6 → K1); the equipment-viewer goldens join the reproduction chain with a committed dataset fixture (F-7 → C1); the rest of the triage runs as classified. |
| 2026-09-28 | Operator away overnight: autonomous mode (orchestrator decides open questions with evidence, logged as "orchestrator decision (delegated)"; no commits). Wave 2 launched: R0, A4, L1, Y1, D2. |
| 2026-09-28 | Test-only agents launched early (they edit no `src/`, so no rebuild contention): C1, C2, Q1, Q2. K1 split into K1a (API: equipment routes development-only + into `routes.rs`, `Query` envelopes, T-951, T-1216) and K1b (tooling: T-1014, T-1012, T-1105, T-1126, equipment definition READMEs) — orchestrator decision (delegated), to keep each under budget and avoid file overlap with A4. |
| 2026-09-28 | Y1 done: tracked symlinks fingerprint by tagged link text; untracked, escaping and dangling links refused; `api_readiness` 60 passed; the judge-only readiness run now completes (FAIL: 91 checks without receipts, 73 unmet requirements — none about fingerprints). K1b launched. D2 done: frontend 1,795 passed, 1 failed (`doc_audit`, L1 in progress); developer-tools 265 passed; DTO `@contract` tags; frontend README golden lines corrected. D3 launched. |
| 2026-09-28 | A4 done: 18 failpoints placed in 13 production files; shared `tests/failpoint_and_race_support/` + `failure_injection_self_checks` (5 cases); lib 531; the 10 affected suites unchanged green (81 passed); release `api` has 0 catalogue strings. Launched K1a, X1, X2, F1, F2 (parallel: shared support built once). |
| 2026-09-28 | K1b done: T-1014 (seed psql stops on the first failed statement), T-1012 (help lists no make targets), T-1105, T-1126 (unknown `--only` slug and empty runs exit 2) — 7 red-first tests; xtask `commands::db` 36/36, developer-tools `dom_oracle` 20/20; READMEs for the two equipment definition folders. Leftovers V-F22..V-F24 queued for E1/G1. |
| 2026-09-28 | C1 done: `contract_parity_goldens` (5 cases, support in `tests/contract_parity_support/`): 51/51 JSON goldens and 2/2 event-stream goldens reproduce from the seed; schema validation and generated-type decoding red only on the registry rows (V-F25 → A3b). Equipment-viewer goldens are trimmed samples of a local 4.7 GB dataset (V-F26): orchestrator decision (delegated) — keep the operator's "include" decision and give agent E3 a hard feasibility stop before any NOTE. Q3, Q4, A3b, E3 launched. |
| 2026-09-28 | Q1 done: `session_authority_properties` — `protected_actions_require_effective_session_authority` and `refresh_replay_revokes_concurrently_issued_successor` each 256/256 at seed 2026092201 (6 s); perturbations (revocation check removed; replay revocation removed) red with shrunk counterexamples; no findings. |
| 2026-09-28 | C2 done: `contract_parity_mod_wire` 6/6 twice (structs, body builders, schema-enum constants, every API DTO cites a contract, roster wire version, mission schema window); 30 mod scripts, comment lines only (`partial` projections declared, F-4 fixed red-first); `mod compile` 0 TBD warnings, `enfusion-comments` 0, `ci-local-schema` 408 citations. T-1088 closable. |
| 2026-09-28 | A3b done: closed `RegistryItemRow`/`RegistryCompatRow` in `arsenal-envelopes.schema.json`; `contract_parity_goldens` 5/5 (runs 1, 2, 4; run 3 saw another agent's transient perturbation); `ci-local-schema` 408 citations. |
| 2026-09-28 | R0 done: `route_acceptance_support/` (runtime route table over 170 routes, table-driven specs with derived probes, actors, runner, contracts), `route_acceptance_coverage` 9/10 (only exactly-one-spec red, awaiting R1–R6), identity_and_core part 6/8 (red only on real handler defects V-F35/V-F36, queued for K2). R1–R6 launched. |
| 2026-09-28 | X1 partial: `controlled_races_reservations` (2) and `controlled_races_identity` (3) pass once (last seat, assignment/withdrawal, refresh winner + replay, replay after successor use, linking — both orders); the permission classifier denied its lock-removal perturbation and its reruns (V-F39); determinism reruns move to the orchestrator gates. |
| 2026-09-28 | D3 done: 15 new goldens for consumed routes (68/68 reproduce on two fresh captures), `contract_parity_goldens` 5/5, frontend `r_api` 107/107 (+8 cases). 21 write routes answer server-generated ids, secrets or request-time timestamps (V-F40): orchestrator decision (delegated) — capture them with the spec's documented normalisation table (format-checked placeholders only) → D3b after D4. D4 launched (+ `SavedFire` DTO move). |
| 2026-09-28 | E3 done (operator decision F-7 fulfilled): `contract_parity_equipment_viewer` 2/2 twice — the production importer builds the viewer index from a 97 KB committed fixture and all 12 development-only routes reproduce their goldens (the six `contracts_v2` positive fixtures replaced by captures; frontend parity 6/6 unchanged); 0 production lines changed. |
| 2026-09-28 | Q2 done: `mission_artifact_properties` — `approval_and_deployment_share_immutable_artifact` 256/256 twice (15–18 s), every outcome class reached; its perturbation was denied by the permission classifier (V-F47, not run). |
| 2026-09-28 | F2 done: 12 `failure_injection_*` cases in four binaries (telemetry 2, fleet 5, audit 3, Discord 2), 12/12 twice; the half-open-listener case evidences T-944's second half; perturbation (publication worker gives up on error) red. It restored the perturbed worker with `git checkout -- <file>` (forbidden by the brief; the file had no other changes); the orchestrator re-verified all 19 failpoint call sites intact. |
| 2026-09-28 | F1 done: 10 `failure_injection_*` cases (identity 4 incl. the documented family revocation after a lost refresh answer, operations 2, missions 4 incl. the relayed deployment call site), 2 runs green; perturbation (reservation before-commit failpoint moved after commit) red with the changed-row diff; restored by sha256. |
| 2026-09-28 | L1 done: `verification_core::repository_laws` (file length, sibling tests, exemption mechanisms, cargo manifests, crate directions, engine layers; 143 verification-core tests); xtask `verify file-length` and `verify engine-layers` delegate with byte-identical output except T-1055 (rules 4 and 7 now cover `editing`); `engineering_laws` 10/10 twice (1.5 s); 5 perturbations red; the frontend `doc_audit` grandfather table removed (T-1041) and the other agent's 100 undocumented items documented (F-1) → `doc_audit` 11/11. |
| 2026-09-28 | K1a done: the 12 equipment routes live in `community_content/routes.rs`, registered only in development (operator decision F-6); `verify route-tags` PASS 165/165; 12 bare `Query` sites answer the error envelope; T-951 listener registry pruned; T-1216 docstring — 18 red-first tests. K2 launched for the remaining envelope defects (V-F35, V-F36, V-F53). |
| 2026-09-28 | Q3 done: `telemetry_revisions_contribute_exactly_once` and `command_executor_fencing_preserves_observed_outcomes` 256/256 twice (every verdict class reached; 6–25 s); perturbations (duplicate applied; fencing comparison dropped) red with shrunk counterexamples. |
| 2026-09-28 | X2 done: 4 `controlled_races_*` cases (approve vs reject, duplicate revisions, corrected revisions in either order, inverted audit commit order), 3 runs green; perturbations on the mission, match and publication-state locks red (the review-row lock alone is redundant with the mission-row lock, V-F56). T-944 is evidenced in both halves with F2 (V-F58). |
| 2026-09-28 | Q4 done: `audit_publication_preserves_committed_event_delivery` and `reservation_transactions_conserve_slots_and_participants` 256/256 (identical input digests across runs; every required outcome class reached); its perturbations were denied by the permission classifier (V-F59, not run). All seven property records now exist. |
| 2026-09-28 | G1 (closing fixes, batch 1) launched early for ledger items V-F23 (code), V-F24, V-F32, V-F33, V-F45, V-F49 (source comments), V-F50, V-F57; a second batch follows the gates if needed. |
| 2026-09-28 | R3 done: `missions_library` part, 22 routes with all seven dimensions (7 route_acceptance + 1 contract_parity cases); red only on real handler defects (V-F62..V-F68, incl. a hidden-draft bookmark leak) queued for K3 after the R wave. |
| 2026-09-28 | R1 done: `operations_events` part, 24 routes (7 + 1 cases); unauthorized (69 probes), ownership, guest and ban green; red only on framework gaps (V-F70, V-F71) and handler defects (V-F72, V-F73, K2's JSON set) queued for K3. |
| 2026-09-28 | R5 done: `fleet_and_telemetry` part, 24 routes (7 + 1 cases, ~12 s); own probes green; red only on derived-probe handler defects (V-F75..V-F78) and the timestamp round-trip gap (V-F79), queued for K3. |
| 2026-09-28 | R6 done: `administration_center_content` part, 50 routes (7 + 1 cases; 100 malformed, 91 boundary, 38 guest probes; 46 authorized exchanges) incl. the development-only equipment routes; red only on the timestamp round-trip gap, K2's JSON class and the refusal-only PATCH users route (V-F81: orchestrator decision (delegated) — explicit refusal-only success shape in the framework + retirement ticket). K2 amended with every JSON-body envelope defect (V-F82). |
| 2026-09-28 | D4 done: typed `ModpackMod`, `Announcement`, `LeaderboardRow`, `SavedFire` (moved from the mortar page) with `@contract` tags; modpacks, announcements, CMS announcements, leaderboards and dashboard announcements claim every key; frontend 1,810 passed, 0 failed (two runs); trunk release ok. D3b launched. |
| 2026-09-28 | R2 done: `operations_reservations` part, 13 routes (7 + 1 cases); red only on K2's JSON class and V-F84/V-F85 (fire-mission event existence, T-1177; malformed slot id) queued for K3. |
| 2026-09-28 | K3 split (orchestrator decision (delegated)): K3a (route-acceptance framework: timestamps compared as instants with the wire format still asserted, top-level array contracts, an explicit refusal-only success shape with a documented reason) launched now; K3b (handler defects V-F62..V-F66, V-F72, V-F73, V-F75, V-F76, V-F84, V-F85 and a shared path-rejection envelope) after K2 and R4. |
| 2026-09-28 | K2 done: every JSON body and query in the API goes through `from_json_rejection`/`from_query_rejection` (≈37 handler files incl. the development-only equipment viewer; the game-runtime session body; the malformed OAuth callback redirects); 34 `contract_parity_json_rejections_*` + 12 equipment query cases; five existing suites moved from the old flat 400 pins to the spec statuses (status assertions kept, messages no longer claim a wrong field); lib 532; route-tags 165/165. |
| 2026-09-28 | G1 done: V-F23 (code), V-F24, V-F32, V-F33 (new `contract_parity_registry_row_constraints_match_the_catalogue_schemas`), V-F45, V-F49 (source comments), V-F50, V-F57 fixed; xtask db/ci/selftest 50/50; `ci-local-schema` 420 citations; engine-layers PASS with unchanged pins. Leftovers V-F89..V-F94 queued (E1/G2). |
| 2026-09-28 | K3b launched (after K2) for the handler defects V-F62..V-F66, V-F72, V-F73, V-F75, V-F76, V-F84, V-F85 and one API-wide path-rejection envelope; R4's findings, when it reports, go to G2. |
| 2026-09-28 | K3a done: round trip compares `date-time` fields as instants while asserting the API's own spelling; `Contract::schema_items` for top-level arrays; `refusal_only` success shape with a documenting-module reason (PATCH admin/users); coverage 14/14 (+4 framework cases); parity deterministic on 3 runs; the 16 remaining reds are exactly K3b's handler list. |
| 2026-09-28 | D3b done: goldens for the 21 write routes with server-generated ids, secrets or request-time timestamps; 43 normalised fields in the design note's normalisation table (7 kinds, each format-checked; a secret golden stores only its placeholder); `_index.tsv` 89 rows; `contract_parity_goldens` 6/6 twice; frontend `r_api` 127/127 (+16). |
| 2026-09-28 | K3b done: `core/http/path_parameters.rs` (`PathParams<T>` + `ApiError::from_path_rejection`) replaces all 97 `Path<…>` extractors in 49 handlers; bookmark writes and `scope=bookmarked` honour mission visibility (information leak closed, T-1173); armory quantity, unknown event scope, unknown roster account, status-stream and commands 404s, fire-mission event existence (T-1177 existence part) and malformed slot ids fixed; all 16 red probes green, 20 existing suites green, lib 536, route-tags 165/165. Early full `db test-it` started. |
| 2026-09-28 | Frontend gate: `cargo test -p website-frontend` 1,826 passed, 0 failed; `identity_browser_session_transactions` 9/9 (`g-frontend-test.log`). The early full `db test-it` failed to build: `/home` reached 100% (the shared host target `~/.cache/tbd-target` had grown to 209 GB with 123 GB of stale website-api test executables and 60 GB of incremental cache; Postgres could not save container state). Orchestrator decision (delegated): removed only rebuildable caches (`debug/incremental` and the website-api test executables in `debug/deps`); `/home` back to 165 GB free, database healthy, the failed run's database dropped; repository (Disk_2) unaffected. Full `db test-it` re-run. |
| 2026-09-28 | Gate run 1 (`g-db-test-it-1.log`): 1,294 passed, 2 failed (build 3 min 27 s, tests 209 s over 148 binaries; V prefixes route_acceptance 66, contract_parity 79, controlled_races 9, failure_injection 27, engineering_laws 10; 29 property records). Both failures are V fallout (V-F103). Frontend clippy wasm32 exit 0 (6 warnings in changed files, V-F102); frontend fmt drift only in the other agent's data_viewer files (V-F101); xtask 1,027 passed, 3 failed (V-F104). G2 launched with every queued small fix. |
| 2026-09-28 | E1 split (orchestrator decision (delegated), ledger size): E1a (READMEs incl. api_v2 test map and `failpoints` feature, design note to implemented state, `@contract` grammar in the standards, stale engine-layer and seed references, READMEs for the other agent's committed equipment folders) and E1b (ship/close/note tickets, known bugs, ticket sync) launched. R4 asked to wrap up (no edits since 02:13; its binary passes). |
| 2026-09-28 | G2 done: gate-run-1 failures fixed (415 expectation; equipment GET routes skipped from the Postgres null sweep with a reason); the other agent's `legacy_archive` concept renamed `unversioned_export_archive` (history rule green; on-disk names changed, V-F108); gameplay test race fixed (shared cwd lock); fmt clean on five crates; clippy clean (api, xtask, verification-core, map-engine, wasm32 touched files); engine-layers output byte-identical with `text::gpu` removed; participants world seeded (array check no longer vacuous, perturbation red); xtask 1,029 passed (1 red until the new files are committed); frontend 1,826. |
| 2026-09-28 | Static gates (`target/api-progress-checkpoint/2026-09-28-v/g-*.log`): route-tags PASS 165/165; file-length 0 violations over 4,103 files; enfusion-comments 0; engine-layers PASS; schema-codegen leaves the tree unchanged; ci-local-schema PASS (420 citations); mod compile clean (0 TBD warnings); release `api` build carries 0 of the 18 failpoint names and no `failpoint` string while a test binary carries them. |
| 2026-09-28 | E1b done: shipped T-1041, T-1026, T-1012, T-1105, T-951, T-1055, T-944, T-949 (evidence notes); T-1014, T-1088, T-1126, T-1173, T-1216, T-1218 filled but not yet shipped (`ticket ship` refuses while the stamp gap keeps `ticket check` red — ship after the commit is stamped); cancelled with proof T-948, T-986, T-933, T-906, T-953; notes on T-1177 and T-1131; 18 new idea tickets T-1229–T-1246 for the NOTE findings; `ticket check` red only on the 16 expected ship-before-stamp errors. |
| 2026-09-28 | E1a done: 46 Markdown files (29 new): api_v2 README (`failpoints`, verification-suite map, configuration rows), core/seeds/workers READMEs, design note at the implemented state (499 lines, normalisation table byte-identical), `@contract` grammar in the documentation standards, engine-boundary and seed/runbook references, 27 READMEs for the other agent's committed equipment folders; remaining_milestones follow-up measured (all six EnfScript files ≤ 500). 67 readme-coverage gaps + 2 link breaks remain in other committed folders (V-F111) → E1c launched. G3 launched for V-F105..V-F107, V-F109. `rust-api-container` started for leptos-gates. |
| 2026-09-28 | G3 done: three more handlers on `from_query_rejection` (5 red-first cases, `query_rejection_envelopes` 30/30); upload 413 comment corrected; data_viewer `aria-expanded` renders "true"/"false"; stale `text::gpu` history comments in both engines rewritten; clippy (api all-targets, engines native + wasm32) clean. Trunk release build started. |
| 2026-09-28 | Final full `db test-it` (`g-db-test-it-final.log`): 1,301 passed, 1 failed (E3's fixture-coverage case now also saw E1a's `positive/README.md` → micro-fix G4); lib 536; 148 binaries, tests 234 s, build 78 s. Trunk release success. `ci-local-schema` re-run PASS after the fixture READMEs. |
| 2026-09-28 | `mk leptos-gates` run 1 (`g-leptos-gates.log`): DOM oracle 15/25; the 10 divergent routes (dashboard, approvals, audit, servercontrol, deployments, events, eventhub, missions, missionview, settings) all trace to golden content now reproduced from the seed (2030 dates and weekdays, countdown years, seeded audit rows, the c000-004 briefing, library order, attendance 50); the five D4 typed-DTO pages matched unchanged. Each divergence was screenshot-audited and accepted with a note (`g-accept-*.log`); one seed-realism quirk noted (V-F113: an upcoming event shows ATTENDED). Re-run started. |
| 2026-09-28 | G4 done (fixture-coverage case counts `*.json` samples only; stray-sample perturbation red). `mk leptos-gates` re-run (`g-leptos-gates-2.log`): exit 0, gate doctor OK, editor suite 21/21, DOM oracle 25/25. Clippy `-D warnings` clean: website-api all targets and `--lib --bins` (failpoints off), xtask + verification-core all targets; `fmt --check` clean on website-api, xtask, verification-core, developer-tools, map-engine, graphics-engine. |
| 2026-09-28 | R4 stopped by the orchestrator (no report after the wrap-up request; its last message says both perturbation attempts were refused by the permission classifier). Its `missions_reviews` part (spec 515 lines, world 473, binary) is complete per coverage and passes in the orchestrator's full runs (V-F114). |
| 2026-09-28 | Frontend final (`g-frontend-test-final.log`): 1,826 passed, 0 failed; `identity_browser_session_transactions` 9/9. Orchestrator perturbation re-checks (each restored, sha256 equal): a changed `GET__factions.json` value → `contract_parity_every_frontend_golden_is_reproduced_by_the_seeded_api` + index case red; a 501-line production file → `engineering_laws_production_files_stay_within_500_lines` red; a removed `@route` tag → `route_acceptance_route_table_matches_every_route_tag` red; the logout failpoint moved after commit → `failure_injection_logout_before_commit_keeps_the_session_and_a_retry_logs_out` red; `decide_revision` applying a duplicate → `telemetry_revisions_contribute_exactly_once` red. Race re-check not attempted (the classifier refused lock-removal edits to X1; X2's four lock perturbations are the race evidence). |
| 2026-09-28 | E1c done: README coverage for the other agent's committed tbd-export, data_viewer, equipment DTO, developer-tools and xtask folders (readme-coverage 67 → 21 violations; link-check 0 breaks); the remaining 19 are the Workbench-generated `Gameplay/Policy/Generated/` folders whose generator refuses foreign files (V-F115, orchestrator decision (delegated): noted for the operator, not renamed unattended); 2 clear when V's deletions are staged. Final full `db test-it` (run 2) started. |
| 2026-09-28 | Final gates: `db test-it` 1,302 passed, 0 failed, 0 ignored (all V prefixes and 7 property records 256/256; lib 536); ci `rust-test-it` 1,302/0; rust-fmt, rust-build, wasm-ci (1,493) PASS; xtask api_readiness 60, property configuration 8, verification-core 144, developer-tools 269; `ci ci-local` stops at verify-no-python on V's 7 unstaged deletions (resolve at commit); documentation gates fail only on V's uncommitted files and V-F115. H1 launched. |
| 2026-09-28 | H1 done: register minimums raised to the measured counts and the V implementation paths added (the register validates); `verification_findings.md`; this checkpoint and `remaining_milestones.md` at V complete; the milestone C record archived; T-1247 to T-1251 filed for V-F110, V-F112, V-F113, V-F115 and V-F116 (written in the minted shape, because `ticket add` refuses while `ticket check` is red on the stamp gap) and `ticket sync` run; judge-only `verify api-readiness` FAIL with no receipts (73 violations, 91 checks did not run). |
| 2026-09-28 | Committed as bd6ec3edf (752 files); the fourteen V tickets shipped and stamped (`ticket check` OK). Post-commit (`pc-ci-*.log`): verify-no-python and verify-no-shell PASS; `ci verify-documentation` fails only on the 19 Workbench-generated `Gameplay/Policy/Generated/` folders (T-1250, operator decision). |

### Milestone V launch amendments

| Agent | Addition to the pre-written prompt |
|---|---|
| brief (all) | Tree clean at P0 (other agent committed all work): the foreign set is whatever uncommitted change V did not make, re-snapshotted per wave; files the other agent committed are ordinary code, still edited only by their owning V agent. |
| A2 | `Cargo.toml` is no longer foreign (committed by the other agent); own hunks only. |
| A3 | The committed equipment routes and `equipment-data-viewer` schemas are inventoried like any route (schemas read-only); write the inventory first with `pending:` rows so R0 can start. |
| D1 | Trees are clean; captures compile committed code only. |
| R0 | Parser handles both the nested equipment router and K1's later dev-only rows in `routes.rs`; known expected-red envelope gaps (12 bare `Query` handlers, K1 fixes); keep specs data-driven. |
| A4 | Read `failpoints_api.md` instead of the module source; re-run the release strings probe after placing call sites; do not touch `audit_notifier.rs` or `Query` extractors (K1). |
| L1 | Dependency law corrected (orchestrator decision (delegated), evidence: the `website-map-engine` line in `api_v2/Cargo.toml` is documented as mission-domain only): website-api depends on neither graphics-engine nor frontend; the frontend does not depend on website-api. Also owns F-1 (100 `doc_audit` doc lines in the other agent's committed files, operator-approved) and T-1055. |
| Y1 | Baseline `api_readiness` 56 passing. |
| D2 | Owns the editor-smoke and arsenal expectation lines for the new registry golden, the routes.csv attendance note, and the now-committed frontend README golden lines. |
| C1 | Run the production audit publisher before reading the SSE `ready` frame (D1 note); operator decision F-7: equipment-viewer goldens reproduced from a committed dataset fixture under a development configuration. |
| C2 | F-4 (fleet-command arguments pointer) red-first; verify T-1088 (all tagged) and cover every tagged class. |
| Q1, Q2 | Binary ≤ 60 s: shared state, unique ids per case. |
| D3 | C1's binary may not exist yet: prove by second-capture diff, then run C1's binary when it lands; skip equipment routes (C1) and the four Value-row DTOs (D4); report added frontend cases. |
| K1a | Keep A4's `fail_point!` calls in place; mirror the `/auth/dev-login` gating so R0's parser reads the dev-only rows. |
| X1, X2, F1, F2 | Read `injection_support_api.md`; placement files named; X2/F2 name the cases that evidence T-944's two halves; F1 does not duplicate the inert-until-armed case; F2 asserts the documented Discord lease-expiry recovery and the 204 claim-after-commit behaviour. |
| C2 | (message) Fix own clippy `collapsible_if` before reporting. |
| K1a | (message) Its `Result<Query>` change broke `tests/leaderboards_paging.rs` direct calls — fix them and include that binary in its checks. |
| A3b | New small agent (orchestrator decision (delegated)): closed registry row contracts for V-F25. |
| E3 | New agent for operator decision F-7 with a hard feasibility stop (≤ 60k tokens reading the importer). |
| R1–R6 | R0 facts (worlds mounted by `#[path]`; `/me*` covered by R0); known handler defects are expected red and belong to K1a/K2; R6: equipment routes development-only, verify the PATCH users 409. |

### Milestone V findings

Every V finding, its class and its outcome are in [verification_findings.md](verification_findings.md).

## Milestone C execution record

The milestone C execution record and launch amendments are archived verbatim in
[milestone_c_execution_record.md](/documentation_v2/archive/api_v2_completion/milestone_c_execution_record.md).
