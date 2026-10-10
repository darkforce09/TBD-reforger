**Status:** live

# API design notes

The design notes of the website [API](/documentation/glossary/a_to_f.md#api): one subject each,
stating what a domain's transactions, locks and refusals must do and naming the tests that hold
them, plus the staging receipts' design. Developers changing a domain's behaviour read them.

## Contents

```text
documentation/crates/api/api_server/design_notes/
├── administration_and_content.md       personnel pages, the audit stream, vehicles, wiki revisions, uploads
├── event_administration.md             the locks every event change takes, and in what order
├── event_eligibility_allocation.md     event access, visibility, pools, promotion and derived attendance
├── fleet_command_ledger.md             the fleet command ledger: commands, states, rules, executors
├── game_ballistics.md                  flight model, calibration, catalogs, fire missions, offline page
├── identity_transactions.md            session authorization, Discord observations, linking, attribution
├── live_occupancy.md                   live slot occupancy and player deployment authorization
├── machine_credentials.md              per-server machine credentials and the runtime-session fence
├── mission_artifacts.md                mission artifacts, reviews, approval and mission deployments
├── reservation_attendance.md           reservation state kept apart from attendance, and corrections
├── reservation_mutation_guards.md      reauthorization and capacity inside the six reservation writes
├── reservation_transaction_design.md   the reservation transaction design and its implementation state
├── staging.md                          staging receipts: fleet, Discord and load procedures, witness rules
└── telemetry.md                        match identity, results revisions, detailed events, fleet status
```

## How it works

Each note covers one subject: the semantics chosen, the lock order, the refusals with their codes,
and the tests that hold them. The identifiers in a note's opening sentence (such as
`fleet_fleet_commands`) name the behaviours it covers.

| Domain | Notes |
|---|---|
| [identity and access](/documentation/glossary/g_to_m.md#identity-and-access) | `identity_transactions.md` |
| [operations](/documentation/glossary/n_to_z.md#operations) | `event_administration.md`, `event_eligibility_allocation.md`, `reservation_attendance.md`, `reservation_mutation_guards.md`, `reservation_transaction_design.md`, `live_occupancy.md` |
| [missions](/documentation/glossary/g_to_m.md#missions) | `mission_artifacts.md` |
| [server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure) | `machine_credentials.md`, `fleet_command_ledger.md` |
| [match telemetry](/documentation/glossary/g_to_m.md#match-telemetry) and [command center](/documentation/glossary/a_to_f.md#command-center) | `telemetry.md` |
| [administration](/documentation/glossary/a_to_f.md#administration) and [community content](/documentation/glossary/a_to_f.md#community-content) | `administration_and_content.md` |
| [operations](/documentation/glossary/n_to_z.md#operations): fire missions and ballistics catalogs | `game_ballistics.md` |
| staging: the fleet, Discord and load receipts | `staging.md` |

### Reading notes

The notes describe the code at the time each was written; where the code has moved since, the
code wins:

- `mission_artifacts.md:56` spells the artifact path parameter `{artifactId}`; the route is
  `/api/v1/missions/{id}/artifacts/{artifact_id}` (`crates/api/api_missions/src/routes.rs`).
- `mission_artifacts.md:81-92` lists the deployment refusals without the codes
  `SERVER_INACTIVE` and `EVENT_MISSION_NOT_ON_SERVER`, which the code returns for an inactive
  server and for an event mission that does not fit the server
  (`crates/api/api_missions/src/services/mission_deployments/deployment_requests.rs`; the
  [mission deployments README](/crates/api/api_missions/src/services/mission_deployments/README.md)
  has the full table).

## Code

- [API crate](/crates/api/api_server/) — the domains the notes describe.
- [Staging procedures](/tools/commands/staging_procedures/) — the harness that records the
  staging receipts `staging.md` designs.

## Boundaries

- Depends on: the API code and its test suites, which the notes describe.
- Used by: the READMEs of the API domains, workers and services, the glossary and the frontend
  administration docs, which link the notes.
- Rules: the files keep their names, since the READMEs link them.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — the domains and routes the
  notes cover.
- [API decisions](/documentation/crates/api/api_server/decisions.md) — the cross-domain decisions the
  notes build on.
