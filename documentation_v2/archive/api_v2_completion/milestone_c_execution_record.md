**Status:** archived — see [the API v2 verification checkpoint](/documentation_v2/website/api_v2/verification_evidence/progress_checkpoint.md)

# API v2 completion — milestone C execution record

The phase handoffs and launch amendments of milestone C (administration and content), moved
verbatim from the verification checkpoint when milestone V closed on 2026-09-28.

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
| 2026-09-27 | ci-local steps so far: verify-editorconfig FAIL (3,205 findings, all in the other agent's untracked assets_v2/equipment folder); verify-no-python / verify-no-shell FAIL only on 9 tracked-but-deleted paths (8 C deletions awaiting `git rm` at commit + the other agent's `apps/mod/.mcp.json`), banned paths none; verify-no-node, verify-ci-shell, verify-engine-layers, verify-staging-compose-paths, verify-mission-rest-size-limits PASS. |
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
