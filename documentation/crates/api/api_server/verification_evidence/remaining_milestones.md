**Status:** live

# API v2 — remaining milestones after M

This document lists the milestones of the API v2 completion program that follow E, F and M:
T, C, B, V and S. Each one opens with a short design note in this folder and closes with its
register entries in `requirements.json` and a checkpoint in `progress_checkpoint.md`. Nothing
here is optional, and readiness stays fail-closed until every listed receipt is current. No
TLA+, TLAPS, TLC or formal-proof toolchain is part of this work.

The scope is set by `completion_plan.md` and `requirements.json`; the current resume point is
`progress_checkpoint.md`.

Game ballistics (B) ran out of the T, C, V, S sequence by operator decision (2026-09-23), in its
own phase with a redesigned approach; it is implemented and verified 2026-09-28 (design in
`game_ballistics.md`).

## Status of the remaining checks

The status comes from the last full `cargo xtask db test-it` (1,339 cases, 2026-09-28; B rows,
with the map-engine filter run and the two B gates), the V run (1,302 cases, 2026-09-28; V rows,
with `cargo xtask mk ci-local-leptos` for the browser session row), the 2026-09-27 run (1,075
cases; C rows), the 2026-09-26 run (877 cases; T rows) and the 2026-09-23 run (784 cases; the
other rows).
"Missing" means the register's case pattern matched no passing test. The pattern shorthand
`name*` means a test whose name starts with `name`.

| Milestone | Requirement | Check | Command | Minimum | Case pattern | Status |
|---|---|---|---|---|---|---|
| T | telemetry_match_identity | telemetry_match_identity | db test-it | 14 | `match_identity*` | passing (14 cases, 2026-09-26) |
| T | telemetry_telemetry_revisions | telemetry_telemetry_revisions | db test-it | 7 | `telemetry_revisions*` | passing (7 cases, 2026-09-26) |
| T | telemetry_telemetry_corrections | telemetry_telemetry_corrections | db test-it | 7 | `telemetry_corrections*` | passing (7 cases, 2026-09-26) |
| T | telemetry_telemetry_atomicity | telemetry_telemetry_atomicity | db test-it | 6 | `telemetry_atomicity*` | passing (6 cases, 2026-09-26) |
| T | telemetry_detailed_events | telemetry_detailed_events (+ schema_quality) | db test-it | 9 | `detailed_events*` | passing (9 cases, 2026-09-26) |
| T | telemetry_telemetry_queue | telemetry_telemetry_queue (+ mod_compilation) | db test-it | 6 | `telemetry_queue*` | passing (6 cases, 2026-09-26) |
| T | dashboard_fleet_dashboard | dashboard_fleet_dashboard | db test-it | 6 | `fleet_dashboard*` | passing (6 cases, 2026-09-26) |
| T | dashboard_statistics_recomputation | dashboard_statistics_recomputation | db test-it | 6 | `statistics_recomputation*` | passing (6 cases, 2026-09-26) |
| C | administration_audit_replay | administration_audit_replay | db test-it | 11 | `audit_replay*` | passing (11 cases, 2026-09-27) |
| C | administration_audit_query_recovery | administration_audit_query_recovery | db test-it | 4 | `audit_query_recovery*` | passing (4 cases, 2026-09-27) |
| C | administration_personnel_pagination | administration_personnel_pagination | db test-it | 9 | `personnel_pagination*` | passing (9 cases, 2026-09-27) |
| C | administration_audit_frontend | administration_audit_frontend (+ frontend_quality, browser_acceptance) | db test-it | 6 | `audit_frontend*` | passing (6 cases, 2026-09-27) |
| C | content_vehicle_mutations | content_vehicle_mutations | db test-it | 18 | `vehicle_mutations*` | passing (18 cases, 2026-09-27) |
| C | content_wiki_features | content_wiki_features (+ frontend_quality, browser_acceptance) | db test-it | 15 | `wiki_features*` | passing (15 cases, 2026-09-27) |
| C | content_content_storage | content_content_storage | db test-it | 15 | `content_storage*` | passing (15 cases, 2026-09-27) |
| B | verification_game_ballistics | verification_game_ballistics (+ backend_regression, route acceptance, contract parity) | db test-it | 29 | `game_ballistics*` | passing (29 cases, 2026-09-28) |
| B | game_ballistics_flight_model | game_ballistics_flight_model | test -p ballistics_model | 51 | modules flight_model, wind, angular_units, catalog | passing (51 cases, 2026-10-03) |
| B | game_ballistics_calibration | game_ballistics_calibration | test -p ballistics_calibration | 34 | modules tests, report, tests_catalog_digest | passing (34 cases, 2026-10-03) |
| B | game_ballistics_elevation_wind_dispersion | game_ballistics_elevation_wind_dispersion | test -p ballistics_solver | 64 | modules tests, wind_corrected_aim, dispersion, crest_clearance, tests_* | passing (64 cases, 2026-10-03) |
| B | game_ballistics_elevation_wind_dispersion | game_ballistics_fuze_and_end_to_end | test -p fire_mission_planning | 9 | modules fuze, tests_end_to_end | passing (9 cases, 2026-10-03) |
| B | game_ballistics_elevation_wind_dispersion | game_ballistics_oracle_elevation_and_wind | test -p ballistics_calibration | 8 | module tests_oracle_elevation_and_wind | passing (8 cases, 2026-10-03) |
| B | game_ballistics_battery | game_ballistics_battery | test -p fire_mission_planning | 29 | modules battery, fire_mission, fire_mission_comparison | passing (29 cases, 2026-10-03) |
| B | game_ballistics_wasm_agreement | game_ballistics_shared_solution_cases | test -p ballistics_agreement_cases | 11 | module case_lattice | passing (11 cases, 2026-10-03) |
| B | game_ballistics_wasm_agreement | game_ballistics_solution_wording | test -p fire_mission_planning | 7 | module solution_wording | passing (7 cases, 2026-10-03) |
| B | game_ballistics_wasm_agreement | game_ballistics_wasm_agreement | mk ballistics-wasm-agreement | 32 | `case ballistics_wasm_agreement_*` | passing (32 of 32, bit-identical, 2026-09-28) |
| B | game_ballistics_offline_page | game_ballistics_offline_page (+ frontend_quality, browser_acceptance) | mk mortar-offline-gate | 15 | `case mortar_offline_*` | passing (15 cases, 2026-09-28) |
| V | verification_route_acceptance | verification_route_acceptance | db test-it | 66 | `route_acceptance*` | passing (66 cases, 2026-09-28) |
| V | verification_contract_parity | verification_contract_parity (+ schema_quality) | db test-it | 85 | `contract_parity*` | passing (85 cases, 2026-09-28) |
| V | verification_property_invariants | verification_property_invariants | db test-it | 26 | the named property cases | passing (26 cases; seven V property records at 256 of 256, 2026-09-28) |
| V | verification_controlled_races | verification_controlled_races | db test-it | 9 | `controlled_races*` | passing (9 cases, 2026-09-28) |
| V | verification_failure_injection | verification_failure_injection | db test-it | 27 | `failure_injection*` | passing (27 cases, 2026-09-28) |
| V | verification_engineering_laws | verification_engineering_laws; repository_quality (`ci ci-local`) | db test-it | 6 | the six named `engineering_laws_*` cases | not run |
| V | verification_engineering_laws | verification_dependency_boundary_laws | test -p repository_laws | 9 | the named `crate_tiers_*` and `crate_firewalls_*` cases | not run |
| V | identity_refresh_rotation | identity_browser_session_transactions | mk ci-local-leptos | 9 | the named browser session cases | passing (9 cases, 2026-09-28) |
| S | staging_fleet | staging_fleet | external | 50 | `case staging_fleet*` | not run |
| S | staging_discord (also on the identity Discord requirements) | staging_discord | external | 13 | `case staging_discord*` | not run |
| S | staging_load | staging_load | external | 10 | `case staging_load*` | not run |

Take the exact case names from `requirements.json` when implementing: several checks name
their cases in full, and the register is the source of truth.

## T — Telemetry

**State:** implemented and verified 2026-09-26; design in `telemetry.md`, evidence in
`progress_checkpoint.md`.

**Covers:**
- telemetry_match_identity, telemetry_telemetry_revisions, telemetry_telemetry_corrections,
  telemetry_telemetry_atomicity, telemetry_detailed_events and telemetry_telemetry_queue.
  telemetry_heartbeat_fencing is already done in E7.
- T-940.13 (see `documentation/tickets/plans/t-940_13_plan.md`).
- dashboard_fleet_dashboard and dashboard_statistics_recomputation.

**Required behavior:**
- **Match identity.** Ingestion is scoped to the machine-authenticated server, and a stable
  server-scoped source-match identity is registered before any report about it is accepted.
- **Revisions.** Revisions carry payload digests:
  - a duplicate retry is inert;
  - the same revision with a different digest answers 409;
  - an older revision never overwrites newer facts;
  - a higher revision may lower counters, and a finalized match stays finalized.
- **Batches.** The whole batch is validated before anything is persisted. A batch and its
  aggregate updates are one transaction, a retry cannot double count, and an invalid entry
  rejects the batch with an error that names its index.
- **Detailed events.** Combat, medical and vehicle events have stable event IDs and a
  deterministic order, with a contract schema under `contracts/definitions/`.
- **Mod queue.** The mod keeps outbound telemetry in a durable, bounded queue until it is
  acknowledged. Queue failures (backlog, drops, age) are visible in heartbeats and server
  status.
- **Machine credentials.** `POST /api/v1/ingest/link-confirm` and
  `POST /api/v1/ingest/match-results` move from the shared service token to machine
  credentials, and `ServiceAuth` is deleted. `/metrics` and the detailed `/healthz` take the
  operator's `OBSERVABILITY_TOKEN` (decision 2026-09-26).
- **Dashboard.** The dashboard and live status select or aggregate the configured fleet
  explicitly.
- **Statistics.** Link changes and telemetry corrections complete the derived-statistics
  updates synchronously or durably.

## C — Administration and content

**State:** implemented and verified 2026-09-27; design in `administration_and_content.md`, evidence
in `progress_checkpoint.md`.

**Covers:**
- administration_audit_replay, administration_audit_query_recovery,
  administration_personnel_pagination and administration_audit_frontend;
- content_vehicle_mutations, content_wiki_features and content_content_storage;
- T-940.7, T-940.8 and T-940.9.

**Required behavior:**
- Personnel pages use stable ordering and the page shape `{items, page, per_page, total}`,
  capped at 100, with a frontend pager (T-940.7).
- Audit SSE IDs replay across reconnects and slow consumers. Retained history that is no longer
  available produces an explicit reset, and failed reads retry even while the notification
  listener is healthy.
- The audit frontend consumes the live stream and deduplicates its overlap with paged history.
- Vehicles support creation, PUT, PATCH and DELETE with permission and validation parity
  (T-940.8).
- The wiki supports headings H1–H6, links, safe images, tables, checklists and revisions, all
  rendered safely (T-940.9).
- Content handles storage failures and request limits. Ownership, validation, pagination and
  error contracts are tested at the real consumer boundaries.

## B — Game ballistics

**State:** implemented and verified 2026-09-28, not yet committed; design in `game_ballistics.md`,
evidence in `progress_checkpoint.md`. It ran in its own phase, out of the T, C, V, S sequence, by
operator decision (2026-09-23).

**Covers:** verification_game_ballistics, game_ballistics_flight_model,
game_ballistics_calibration, game_ballistics_elevation_wind_dispersion, game_ballistics_battery,
game_ballistics_wasm_agreement and game_ballistics_offline_page; T-940.10, the T-1177 remainder
and T-1245.

**Required behavior:**
- The map-engine game-ballistics module gains elevation, drag, wind, dispersion and battery
  solutions. It is shared, headless, with the offline mortar page.
- Checks cover analytic cases, symmetry, convergence and bounded failure, plus native/WASM
  agreement within 1 mil.
- Calibration fixtures are versioned and derived from the game.

## V — Verification completeness

**State:** implemented and verified 2026-09-28, committed as bd6ec3edf; design in
`verification_completeness.md`, evidence in `progress_checkpoint.md`, findings in
`verification_findings.md`.

**Covers:** verification_route_acceptance, verification_contract_parity,
verification_property_invariants (seven more cases), verification_controlled_races,
verification_failure_injection and verification_engineering_laws (with the
`repository_quality` replay `cargo xtask ci ci-local`). It also covers every identity_* or
administration_* check that still lacks exact cases, including
identity_browser_session_transactions.

**Required behavior:**
- **Route acceptance.** Every registered route is checked for authorized, unauthorized,
  ownership, guest, ban, malformed and boundary requests.
- **Contract parity.** Handler responses, schemas, generated types, frontend DTOs and mod wire
  versions stay compatible. This includes re-baselining the 14 committed frontend goldens under
  `contracts/fixtures/api_goldens/` that `seeds/content_golden.sql` does not reproduce:
  - their dates are past seed rule 4;
  - some were captured from other databases;
  - migration 0025's event trigger leaves one audit row stamped with the wall clock.
- **Properties.** Rust proptest invariants cover authorization policies, quotas, linking,
  sessions, artifacts, telemetry, commands and audit ordering.
- **Controlled races.** Tokio barriers deterministically exercise last-seat refresh, replay,
  assignment, withdrawal, linking, approval, ingest and commit ordering.
- **Failure injection.** Explicit failpoints verify rollback, recovery and the boundaries of
  effect and acknowledgement failures.
- **Engineering laws.** Production files stay under 500 lines and tests under 1,000, tests are
  separate, and the architectural boundaries hold (CLAUDE.md laws 6 and 7, with zero
  exemptions).

## S — Staging

**Covers:** staging_fleet, staging_discord and staging_load.

**Status:** implemented and gated; receipts pending the run day. The harness, the host tool, the
engines and the console command pass their checks in the 2026-09-29 sweep: `staging_harness` (95
cases), `staging_verification_engines` (84), `staging_fixture_tool` (52) and `fleet_console_command`
(7). The three operational checks declare 50, 13 and 10 cases and stay not run until the
recorded runs on the staging host write their receipts.

**Required behavior:**
- Five real staging servers and game clients verify remote controls, identity, and both
  terrain transitions (scenario restart and host restart).
- The real Discord bot and guilds, including a partner guild, verify:
  - healthy role changes;
  - cached outage grace;
  - partner eligibility;
  - the administrator override;
  - non-blocking warnings.
- An xtask load harness runs a reproducible workload:
  - 1,000 accounts and 100 concurrent clients;
  - at least 20 requests per second for 30 minutes;
  - it records the hardware and the request mix;
  - it requires p95 of 500 ms or less for JSON reads and 1 s or less for writes;
  - it measures game operations separately.
- Receipts are structured observations. Any missing external dependency (servers, bot, guilds,
  clients) is named and its checks are left unrun. No evidence is fabricated.

## Follow-up recorded by E, F and M

CLAUDE.md law 7 has no exemptions. The six EnfScript files this follow-up named each stand at or
under 500 lines, measured on 2026-09-28; `cargo xtask verify file-length` scans 4,103 source files
(411 of them `.c`) and reports no violation:

| File | Lines |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Manager/TBD_SpawnManager.c` | 287 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Mission/TBD_MissionLoader.c` | 342 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c` | 377 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Service/TBD_LobbyService.c` | 291 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/API/Results/TBD_ResultsReporter.c` | 186 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminService.c` | 406 |

The [mod script modularisation](/documentation/apps/mod/script_modularisation_progress_checkpoint.md)
program decomposed them by responsibility, verified by `cargo xtask mod compile`; the operator
waived its in-game playtest for the pre-alpha. `cargo xtask verify file-length` and the API's
`engineering_laws` suite hold the ceiling from here on.
