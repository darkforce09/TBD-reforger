# API v2 — remaining milestones after M

This document lists the milestones of the API v2 completion program that follow E, F and M:
T, C, B, V and S. Each one opens with a short design note in this folder and closes with its
register entries in `requirements.json` and a checkpoint in `progress_checkpoint.md`. Nothing
here is optional, and readiness stays fail-closed until every listed receipt is current. No
TLA+, TLAPS, TLC or formal-proof toolchain is part of this work.

The scope is set by `completion_plan.md` and `requirements.json`; the current resume point is
`progress_checkpoint.md`.

Game ballistics (B) is out of the T, C, V, S sequence by operator decision (2026-09-23): it is
done later, in a separate phase whose approach is redesigned there. Its requirement
`verification_game_ballistics` stays registered, so readiness stays fail-closed until that
phase delivers it.

## Status of the remaining checks

The status comes from the last full `cargo xtask db test-it` (784 cases, 2026-09-23).
"Missing" means the register's case pattern matched no passing test. The pattern shorthand
`name*` means a test whose name starts with `name`.

| Milestone | Requirement | Check | Command | Minimum | Case pattern | Status |
|---|---|---|---|---|---|---|
| T | telemetry_match_identity | telemetry_match_identity | db test-it | 1 | `match_identity*` | missing |
| T | telemetry_telemetry_revisions | telemetry_telemetry_revisions | db test-it | 1 | `telemetry_revisions*` | missing |
| T | telemetry_telemetry_corrections | telemetry_telemetry_corrections | db test-it | 1 | `telemetry_corrections*` | missing |
| T | telemetry_telemetry_atomicity | telemetry_telemetry_atomicity | db test-it | 1 | `telemetry_atomicity*` | missing |
| T | telemetry_detailed_events | telemetry_detailed_events (+ schema_quality) | db test-it | 1 | `detailed_events*` | missing |
| T | telemetry_telemetry_queue | telemetry_telemetry_queue (+ mod_compilation) | db test-it | 1 | `telemetry_queue*` | missing |
| T | dashboard_fleet_dashboard | dashboard_fleet_dashboard | db test-it | 1 | `fleet_dashboard*` | missing |
| T | dashboard_statistics_recomputation | dashboard_statistics_recomputation | db test-it | 1 | `statistics_recomputation*` | missing |
| C | administration_audit_replay | administration_audit_replay | db test-it | 1 | `audit_replay*` | missing |
| C | administration_audit_query_recovery | administration_audit_query_recovery | db test-it | 1 | `audit_query_recovery*` | missing |
| C | administration_personnel_pagination | administration_personnel_pagination | db test-it | 1 | `personnel_pagination*` | missing |
| C | administration_audit_frontend | administration_audit_frontend (+ frontend_quality, browser_acceptance) | db test-it | 1 | `audit_frontend*` | missing |
| C | content_vehicle_mutations | content_vehicle_mutations | db test-it | 1 | `vehicle_mutations*` | missing |
| C | content_wiki_features | content_wiki_features (+ frontend_quality, browser_acceptance) | db test-it | 1 | `wiki_features*` | missing |
| C | content_content_storage | content_content_storage | db test-it | 1 | `content_storage*` | missing |
| B | verification_game_ballistics | verification_game_ballistics (+ frontend_quality, browser_acceptance) | db test-it | 1 | `game_ballistics*` | missing — separate later phase |
| V | verification_route_acceptance | verification_route_acceptance | db test-it | 1 | `route_acceptance*` | missing |
| V | verification_contract_parity | verification_contract_parity (+ schema_quality) | db test-it | 1 | `contract_parity*` | missing |
| V | verification_property_invariants | verification_property_invariants | db test-it | 25 | the named property cases | 19 of 25 |
| V | verification_controlled_races | verification_controlled_races | db test-it | 1 | `controlled_races*` | missing |
| V | verification_failure_injection | verification_failure_injection | db test-it | 1 | `failure_injection*` | missing |
| V | verification_engineering_laws | verification_engineering_laws; repository_quality (`ci ci-local`) | db test-it | 1 | `engineering_laws*` | missing |
| V | identity_refresh_rotation | identity_browser_session_transactions | mk ci-local-leptos | 9 | the named browser session cases | needs the frontend lane |
| S | staging_fleet | staging_fleet | external | 1 | `case staging_fleet*` | not run |
| S | staging_discord (also on the identity Discord requirements) | staging_discord | external | 1 | `case staging_discord*` | not run |
| S | staging_load | staging_load | external | 1 | `case staging_load*` | not run |

Take the exact case names from `requirements.json` when implementing: several checks name
their cases in full, and the register is the source of truth.

## T — Telemetry

**Covers:**
- telemetry_match_identity, telemetry_telemetry_revisions, telemetry_telemetry_corrections,
  telemetry_telemetry_atomicity, telemetry_detailed_events and telemetry_telemetry_queue.
  telemetry_heartbeat_fencing is already done in E7.
- T-940.13 (see `docs/plans/t-940_13_plan.md`).
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
  deterministic order, with a contract schema under `contracts_v2/definitions/`.
- **Mod queue.** The mod keeps outbound telemetry in a durable, bounded queue until it is
  acknowledged. Queue failures (backlog, drops, age) are visible in heartbeats and server
  status.
- **Machine credentials.** `POST /api/v1/ingest/link-confirm` and
  `POST /api/v1/ingest/match-results` move from the shared service token to machine
  credentials, and `ServiceAuth` is deleted. `/metrics` and the detailed `/healthz` also use
  `SERVICE_TOKEN` today and need their own authentication decision.
- **Dashboard.** The dashboard and live status select or aggregate the configured fleet
  explicitly.
- **Statistics.** Link changes and telemetry corrections complete the derived-statistics
  updates synchronously or durably.

## C — Administration and content

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

**Phase:** moved out of the T, C, V, S sequence by operator decision (2026-09-23); done later in
its own phase, whose approach is redesigned there. The behavior below records the current
register scope.

**Covers:** verification_game_ballistics and T-940.10.

**Required behavior:**
- The map-engine game-ballistics module gains elevation, drag, wind, dispersion and battery
  solutions. It is shared, headless, with the offline mortar page.
- Checks cover analytic cases, symmetry, convergence and bounded failure, plus native/WASM
  agreement within 1 mil.
- Calibration fixtures are versioned and derived from the game.

## V — Verification completeness

**Covers:** verification_route_acceptance, verification_contract_parity,
verification_property_invariants (6 more cases), verification_controlled_races,
verification_failure_injection and verification_engineering_laws (with the
`repository_quality` replay `cargo xtask ci ci-local`). It also covers every identity_* or
administration_* check that still lacks exact cases, including
identity_browser_session_transactions.

**Required behavior:**
- **Route acceptance.** Every registered route is checked for authorized, unauthorized,
  ownership, guest, ban, malformed and boundary requests.
- **Contract parity.** Handler responses, schemas, generated types, frontend DTOs and mod wire
  versions stay compatible. This includes re-baselining the 14 committed frontend goldens under
  `apps/website/frontend/tests/fixtures/api/` that `seeds/content_golden.sql` does not reproduce:
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

CLAUDE.md law 7 has no exemptions. Six EnfScript files were already over 500 lines before
E, F and M, and E/F/M only keeps them from growing:

- `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionLoader.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/TBD_LobbyService.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminService.c`

Each is decomposed by responsibility in a dedicated pass, verified by `cargo xtask mod compile`
and an in-game playtest.
