**Status:** live

# API v2 verification checkpoint — 2026-09-27

Milestones E, F, M, T and C are implemented and verified; overall readiness is **not passing**,
because milestones V and S have not started and game ballistics (B) is a later phase (see
`remaining_milestones.md`), and `cargo xtask verify api-readiness --execute` was not run (it needs
a quiet working tree). T is committed on main as 0ef292758. C is not committed: the operator
commits it (see the commit-time notes under open items). Do not reset, clean, stash or revert
anything in the working tree: it holds C's uncommitted work and another agent's uncommitted work
(equipment/vehicle export and the equipment data viewer in `tools_v2/xtask`,
`apps/website/api_v2/src/community_content`, the frontend `data_viewer`,
`contracts_v2/definitions/equipment-*` and `assets_v2/equipment`).

## Milestone status

| Milestone | State |
|---|---|
| E — eligibility, allocation, occupancy, no-show, machine credentials, runtime sessions | Complete. |
| F — fleet command ledger, host agent, executors, recovery | Complete. |
| M — artifacts, reviews, approval binding, deployments, authored preservation, workspace | Complete. |
| T — match identity, revisions, corrections, atomicity, detailed events, telemetry queue, fleet dashboard, statistics | Complete (design: `telemetry.md`). |
| C — personnel pagination, audit replay and query recovery, audit frontend, vehicle mutations, wiki features, content storage | Complete, not committed (design: `administration_and_content.md`). |
| V, S | Not started (`remaining_milestones.md`). |
| B | Later separate phase by operator decision (2026-09-23). |

Migrations 0057–0059 are pinned; the next migration is 0060. Versions 0022–0024 stay retired.

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
| Frontend lane (steps run one by one) | fmt diffs only in the other agent's `data_viewer` and equipment DTO parity files (`p7-frontend-fmt-2.log`); clippy wasm32 exit 0, no warning in a C file (`p7-frontend-clippy.log`); `cargo test -p website-frontend` 1,802 passed, 1 failed: `doc_audit`, all 100 findings in the other agent's `apps/website/frontend/src/v2/apps/debug/data_viewer` (93) and `apps/website/frontend/src/v2/core/api/dto/equipment_data_viewer` (7) (`p7-frontend-test-2.log`); trunk release success (`p7-trunk-release.log`) |
| `mk leptos-gates` | exit 0, gate doctor OK, editor suite 21/21, DOM oracle 25/25 (`p7-leptos-gates-2.log`). The first run was 20/25 (`p7-leptos-gates.log`), diverging on exactly the five C routes; each was screenshot-audited and accepted with notes: personnel (pager), audit (live status badge; the fixture stream ends after `ready`, so it settles on Reconnecting), vehicles (administrator controls, compact header), wiki and wikislug (block renderer, revisions panel). The first wikislug accept was reverted because its screenshot showed leaked `view!` source in the revisions pager (an unbraced `disabled=page >= page_count`); G3 braced it and added the `view_attributes_*` guard. The 25/25 run predates G4's backend shutdown change; the oracle is fixture-driven and the frontend is unchanged since |
| `ci ci-local-schema` / codegen freshness | PASS: verify-codegen-fresh PASS, 230 `@contract` citations resolve (`p7-ci-local-schema.log`); a codegen re-run changes nothing |
| `verify route-tags` | 153 registered routes all documented; FAIL only on the other agent's 12 unwired `/api/v1/debug/equipment-data/*` tags (`p7-route-tags.log`) |
| `verify file-length` / `enfusion-comments` / no-select-star | 0 violations over 3,987 files / 0 findings / clean (`p7-ci-verify-coding-standards.log`) |
| Documentation gates | On the committed tree `ci verify-documentation` fails only on C's untracked new files (`p7-ci-verify-documentation.log`). With `--with-untracked`: readme-coverage finds only C's tracked-but-deleted files (cleared by `git rm` at commit) and the other agent's untracked equipment, `improved_layout` and `data_viewer` folders; link-check finds one line, the pre-existing T-1092 checkpoint link in `documentation_v2/mod/script_modularisation_progress_checkpoint.md`; markdown-placement OK (`p7-*-untracked.log`) |
| `ci ci-local` (steps run one by one) | verify-editorconfig FAIL, 3,205 findings, all in the other agent's `assets_v2/equipment/`; verify-no-python and verify-no-shell FAIL only on 9 tracked-but-deleted paths (C's 8 deletions and the other agent's `apps/mod/.mcp.json`), no banned path; verify-no-node, verify-ci-shell, verify-engine-layers, verify-staging-compose-paths, verify-mission-rest-size-limits, rust-build and wasm-ci PASS; rust-test-it is the final `db test-it` above; ci-local-leptos and ci-local-schema as above |
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
| `ci ci-local` | stops at step 1 (`verify-editorconfig`): 3,205 findings, all in the other agent's untracked `assets_v2/equipment/`; the later steps were run individually above |

## Open items

- **Committing C** (the operator commits):
  - `git rm` the 8 files C deleted: `community_content/handlers/media_upload.rs`,
    `vehicle_database.rs` and `wiki_knowledgebase.rs` in `apps/website/api_v2/src/`, and in
    `apps/website/frontend/src/v2/pages/doctrine_and_info/` `vehicles/helpers.rs`,
    `vehicles/tests/vehicles.rs`, `wiki/helpers.rs`, `wiki/markdown.rs` and
    `wiki/markdown_article.rs`. This also clears the readme-coverage, verify-no-python and
    verify-no-shell findings on them.
  - `apps/website/api_v2/src/community_content/models/generated/mod.rs` is the other agent's
    untracked file with C's three `pub mod` lines added: commit a version holding only C's lines
    through a temporary index, and leave the working file as it is.
  - `tools_v2/xtask/src/commands/generate/schema_types.rs` and
    `community_content/models/mod.rs` carry C's hunks next to the other agent's: stage C's hunks
    only. The same holds for the other foreign files listed under the C verification table.
  - `.ai/tickets/wave.lock` holds the repack that `ticket ship` wrote for T-940.7, T-940.8 and
    T-940.9; it goes with the C commit. After the commit, `cargo xtask ticket stamp-sha` each of the
    three with the landing SHA.
- **Tickets.** T-940.13 (detailed events) is still `ready` from T, and T-1222 still has no
  `shipped_at` stamp (`ticket stamp-sha T-1222 0ef292758`). T-950 ("Audit SSE stream has no
  client", `idea`) has its scope delivered by the audit page; whether to ship or close it is the
  operator's decision. T-944 ("Audit stream: id-order race and half-open socket", `queued`) was
  not assessed against C.
- **Legacy audit rows.** Rows stored with a NULL `created_at` before C still read as
  `0001-01-01`; system rows appended since C stamp `now()`.
- **`view!` attributes.** A follow-up task was spawned to brace the unbraced `view!` comparisons
  outside C's pages (C's pages are braced and guarded by `view_attributes_*`).
- **Schema length.** `wiki-page.schema.json` is 764 lines. JSON schemas follow the precedent of
  `mission.schema.json` (1,713 lines) and are outside the `verify file-length` gate.
- **Readiness gate.** `cargo xtask verify api-readiness` stops at fingerprinting on the tracked
  `AGENTS.md` symlink (a root fingerprint input); resolving that is a tooling decision for the
  operator. `--execute` was not run: it also needs a quiet working tree.
- In-engine exercise of match registration, events and results needs a LIVE round with a connected
  admin client (the two-client playtest runbook); the API side is covered by the integration
  suites.
- `apps/website/frontend/tests/fixtures/api/GET__dashboard.json` keeps its pre-existing
  `next_event`/`my_assignment` (the seed's dates are past rule 4); the V milestone re-baselines the
  goldens.
- The dashboard `fleet` and `MatchEventPage` shapes cite `match-telemetry.schema.json`; a dashboard
  response schema is part of V's contract parity.
- **Next:** milestone V, then S, with a quiet working tree for `verify api-readiness --execute`;
  game ballistics (B) follows in its own later phase by operator decision.

## Milestone C execution record

Plan: `/home/Samuel/.claude/plans/pasted-content-id-5ce1-resume-the-compiled-moth.md` (sub-agent
roster P1–P8 with pre-written prompts; shared brief in the session scratchpad `c_brief.md`).

| Date | Phase handoff |
|---|---|
| 2026-09-26 | P0 done: shims, foreign baseline (4,649 files hashed), brief written; P1 (A1, A2, A3) launching. |
| 2026-09-26 | P1 done: design note, five schemas + codegen (ci-local-schema PASS), migrations 0057–0059 pinned (listed suites green); P2 B0 ∥ B1 launching. |
| 2026-09-26 | B0 done (content_url_policy, from_json_rejection, handler directory modules; lib 425); B2 ∥ B3 launched with the central-routes correction. |
| 2026-09-26 | B1 done (personnel page, audit ready/reset/floor, lib 427, admin suites green); C1 ∥ C2 launched. |
| 2026-09-27 | C2 done (personnel_pagination 9, audit_frontend 5); B2 done (vehicles, uploads, announcements; lib 500); C3 launched. |
| 2026-09-27 | C1 done (audit_replay 10, audit_query_recovery 4); B3 done (wiki markup + revisions; lib 501); C4 ∥ D1 launched. |
| 2026-09-27 | C3 done (vehicle_mutations 18, content_storage 14); announcement Query-rejection envelope queued for G1. |
| 2026-09-27 | C4 done (wiki_features 15). P3 complete; P3 phase gate (full db test-it) running while D1 works. |
| 2026-09-27 | P3 gate: full `db test-it` 1,058 passed, 0 failed (`target/api-progress-checkpoint/2026-09-26-c/p3-db-test-it.log`). |
| 2026-09-27 | D1 done (seed revision invariant, goldens, DTO modules, safe_url; r_api 98). P5 F1–F4 launched. G1 queue: announcement Query envelope, system audit `created_at`. |
| 2026-09-27 | G1 done (`from_query_rejection` helper; system audit lines stamp `now()`; content_storage 15, audit_frontend 6, lib 503). G2 queue: T's unformatted `telemetry_url_guard.rs`, wiki revisions onto the helper. |
| 2026-09-27 | F1 done (personnel pager, URL state; personnel tests 37). |
| 2026-09-27 | F4 done (vehicle admin UI; vehicles tests 52). A `git mv` staging slip was restored; index verified equal to session start. |
| 2026-09-27 | F3 done (AST renderer, per-slug article, revisions + restore, editor refusals; wiki tests 55; also updated `core/test_support/pins.rs` wiki include list). |
| 2026-09-27 | F2 done (SSE frame parser, audit stream client, live merge, oracle `.sse.txt`). P5 complete. E1 ∥ G2 launched; P7 frontend lane started (fmt: only foreign diffs). |
| 2026-09-27 | P7 frontend lane: fmt diffs only in foreign data_viewer/equipment files; clippy wasm32 exit 0 with 0 warnings in C files; `cargo test -p website-frontend` 1,795 passed, 1 failed (`doc_audit`, 100 findings all foreign); trunk release success. |
| 2026-09-27 | G2 done (T's `telemetry_url_guard.rs` formatted; wiki revisions on the shared helper). E1 done (api_overview, stale docs, T-940.7/.8/.9 shipped + wave repack). P7 backend: db test-it 1,062/0; lib 503; clippy website-api and xtask clean; ci-local-schema PASS (codegen fresh, 230 citations); route-tags FAIL only on the other agent's 12 equipment-viewer tags (153 routes documented); file-length 0 violations. leptos-gates: editor 21/21, oracle 20/25 (only the five C routes); personnel, audit, vehicles accepted after screenshot audit; wikislug accept reverted (leaked view! text in the revisions pager) → G3. |
| 2026-09-27 | ci-local steps so far: verify-editorconfig FAIL (3,205 findings, all foreign `assets_v2/equipment`); verify-no-python / verify-no-shell FAIL only on 9 tracked-but-deleted paths (8 C deletions awaiting `git rm` at commit + the other agent's `apps/mod/.mcp.json`), banned paths none; verify-no-node, verify-ci-shell, verify-engine-layers, verify-staging-compose-paths, verify-mission-rest-size-limits PASS. |
| 2026-09-27 | G3 done (braced view! attributes in every C page, compact vehicles header, revisions pager fits; `view_attributes_*` guard with mutation proof; dist rebuilt). wikislug, wiki, vehicles accepted after screenshot audit; leptos-gates re-run for the official result. Dev DB: wiki/vehicle seeds applied; 17 temporary `walkthrough-*` users for the paging walkthrough (removed afterwards). |
| 2026-09-27 | leptos-gates re-run: exit 0, editor 21/21, DOM oracle 25/25. Live walkthrough (rust-api-container + spa-gate-serve, dev-login admin): personnel per_page=10 over 22 members (pages 1–3, Next disabled at the end, URL state survives reload); audit stream LIVE, vehicle create/PUT/soft DELETE rows arrived live, rows written while the API was down replayed once after restart; vehicle form refused `javascript:` image URL client-side; wiki-formatting-guide renders H1–H6 with anchors, safe external links (rel noopener noreferrer nofollow), lazy no-referrer image, aligned table, disabled checklist, all callouts, code, quote, rule; two saves → revisions 2, 3; stale editor save → 409 with reload; revision 1 restored → revision 4. Walkthrough users and vehicle removed. Found: graceful shutdown never ends open SSE streams (the stopped API lingered until SIGKILL) → G4. |
| 2026-09-27 | G4 done: `core/process_lifecycle` holds the process-wide shutdown signal, every open event stream closes on SIGINT or SIGTERM and the stopped API exits in about 3 ms; `audit_replay_shutdown` binary. Final backend gates: `db test-it` 1,075 passed, 0 failed; lib 515; clippy website-api re-run clean. |
| 2026-09-27 | H1 done: `requirements.json` (C implementation paths, measured minimums), this checkpoint, `remaining_milestones.md` and the shutdown clause in `api_overview.md`. C is complete and awaits the operator's commit. |

### Milestone C launch amendments

| Agent | Addition to the pre-written prompt |
|---|---|
| brief (all) | Every `cargo xtask …` runs on the host via `<scratchpad>/bin/hostcargo xtask …`; only `mk` recipes, `trunk` and frontend cargo run in the container with `target-container-api-v2` (glibc stamp guard). |
| A1 | Explicit current-layout paths for the path map; the suite file/case-glob list; ≤ 500 lines. |
| A2 | Check typify handles the recursive `type`-tagged block oneOf (restructure the schema, never the generator); TARGETS entries placed before the other agent's equipment block; callout kinds note, tip, important, info, warning, caution, critical. |
| A3 | Backfill/null-legacy details restated; `hostcargo test -p website-api --lib prose`; a hand probe of the floor trigger in a throwaway `c_floor_probe` database. |
| brief (all) | Section "Contract decisions fixed in P1" appended after A2 (refusal detail shapes, finding codes, revision shapes, callout kinds, upload URL pattern, PATCH tri-state, landed migrations). |
| B0 | URL policy must agree with the schema `pattern`s (read them); explicit refusal list incl. mixed-case schemes and `http:` for images; keep existing `@route` tags; README-coverage `--with-untracked` checks for community_content and core. |
| B1 | Read the design note sections and the two schemas; Query rejection answers the standard envelope with 400; ready id = resume_after; `audit_row_stream` yields rows only; no new C suites (C1/C2 own them), sibling unit tests allowed. |
| B2 | Route-tags reads only `<domain>/routes.rs`: remove B0's `vehicle_database::routes()` and list every vehicle route in `routes.rs` (re-read before each edit; B3 edits wiki lines concurrently); design-note/schema reading list; vehicle row `FOR UPDATE` before the audit append; no new C suites. |
| B3 | Same route correction for the wiki; may update the `WikiInput` doc link in `fire_missions.rs`; revisions list paging rules (default 20, clamp 100, 400 invalid); also repairs `community_content_reads.rs` for B2's changes; README in `services/wiki_markup/`. |
| F3, F4, E1 | (queued) `vehicle_database_page.md` (lines 49,59) and `wiki_page.md` (lines 78) name the deleted handler files; the glossary "audit logs" entry (`glossary/a_to_f.md`) says the page does not use the SSE feed. |
| B2, B3 | (message) May add null-tolerance sweep/skip entries for their own GET routes in `tests/null_tolerance_support/` (`every_get_route_is_swept_or_skipped_with_a_reason`). |
| C1 | B1 implementation facts; fallback failure-injection methods if the rename does not fail the read; run each suite twice (flakiness). |
| C2 | B1 implementation facts (`q` escaping, 400 envelope); literal `%`/`_` search case; scope users by a unique `q` prefix; run each suite twice. |
| E1 | (queued) stale shapes in `api_overview.md` (lines 112,237), `personnel_roster_page.md` (lines 72–74,118), `audit_logs_page.md` (lines 65–67) (the latter two also for F1/F2). |
| C3 | B2 implementation facts (400 invalid id, PUT clears absent optionals, audit action names, upload 413 code, non-multipart 400); own copy of the raising-trigger failure injection; `upload_dir` pointing at a regular file for the 503 case; run each suite twice. |
| F4, E1 | (queued) `content_manager_page.md` (lines 101) cites the retired `media_upload.rs`. |
| C4 | B3 implementation facts (404 numeric base on missing page, code language, soft break, bracket callouts, finding codes); extra cases: nesting > 16, unknown body field, one audit row per save, revision-invariant check through the API; own raising-trigger copy. |
| D1 | Seed invariant: every seeded wiki page has a revision row for its current revision (owns `wiki_pages.sql` and `content_golden.sql` for that); formatting-guide article golden; minimal compile fixes allowed in page code after the `AdminUserRow` move (listed). |
| brief (all) | Frontend clippy runs exactly as the lane (no `-D warnings`; ~198 pre-existing warnings) with zero warnings in the agent's own files; section "Frontend facts fixed in P4" appended (DTO module paths, safe_url helpers, goldens, DOM-oracle fixture rule). |
| F1–F4 | No trunk build per agent (the orchestrator builds once in P7); `doc_audit` filter run; oracle fixture names per route. |
| F2 | Deterministic oracle stream: new `GET__admin__audit-logs__stream.sse.txt` (one `ready` frame) + `_index.tsv` row; Offline badge state; also owns the glossary "audit logs" entry. |
| F3, F4 | Own the stale retired-handler lines in their feature docs. |
| G1 | Launched early (parallel with P5) for the two queued backend items; red-before case required; shared `from_query_rejection` helper preferred; later gate failures go to a second closing-fix run. |
| E1 | (queued) `personnel_roster_blueprint/README.md` (lines 30-31) ("the table shows no record count") is stale after the pager. |
| E1 | (queued) `apps/website/frontend/src/v2/pages/doctrine_and_info/README.md` (lines 24,28) ("untyped JSON", "only reads") and `documentation_v2/website/frontend/pages/doctrine_and_info/README.md` (lines 32) are stale after F4 (and F3). |
| E1 | Collected stale-doc list from B0–F4 reports; `--with-untracked` documentation gates; T-950 scope check (report only); foreign `apps/website/frontend/README.md` stale lines reported, not edited. |
| G2 | Launched before the P7 backend gates (telemetry_url_guard fmt from T; wiki revisions onto `from_query_rejection`). |
| G4 | Launched after the walkthrough: SSE streams end on shutdown (process-wide lifecycle signal in core, no `AppState` field — foreign file), `audit_replay_shutdown` test binary, hand proof with SIGTERM; also adds G3's wiki README rule line. |
