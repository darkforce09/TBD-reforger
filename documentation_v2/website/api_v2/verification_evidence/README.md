**Status:** live

# API verification evidence

The acceptance record of the website [API](/documentation_v2/glossary.md#api): the register of
requirements that `cargo xtask verify api-readiness` judges, the acceptance contract behind it,
the program's checkpoint and remaining milestones, and the design notes that state what each
domain's transactions, locks and refusals must do. Developers changing a domain's behaviour and
agents running the readiness check read it.

## Contents

```text
documentation_v2/website/api_v2/verification_evidence/
├── completion_plan.md                  the acceptance contract: what "the API is complete" means
├── event_administration.md             the locks every event change takes, and in what order
├── event_eligibility_allocation.md     event access, visibility, pools, promotion and derived attendance
├── fleet_command_ledger.md             the fleet command ledger: commands, states, rules, executors
├── identity_transactions.md            session authorization, Discord observations, linking, attribution
├── live_occupancy.md                   live slot occupancy and player deployment authorization
├── machine_credentials.md              per-server machine credentials and the runtime-session fence
├── mission_artifacts.md                mission artifacts, reviews, approval and mission deployments
├── progress_checkpoint.md              the completion program's latest verification checkpoint
├── property_test_evidence.md           how property runs count generated cases apart from tests
├── remaining_milestones.md             the milestones T, C, B, V and S still to land
├── requirements.json                   the acceptance register: requirements and the checks that prove them
├── reservation_attendance.md           reservation state kept apart from attendance, and corrections
├── reservation_mutation_guards.md      reauthorization and capacity inside the six reservation writes
└── reservation_transaction_design.md   the reservation transaction design and its implementation state
```

## How it works

The folder holds three kinds of file. They are the evidence the readiness check is bound to, so
they are indexed here and never reworded; a change to what a note says is a new note or a new
register entry.

| Kind | Files | What it holds |
|---|---|---|
| Register | `requirements.json` | version 1; 73 requirements, each with its behaviour, the implementation paths it rests on, its assumptions and the checks that prove it; 91 checks, 82 of class `implementation`, 6 `property` and 3 `operational`, each with its command, timeout, minimum case count, success marker and case pattern |
| Program records | `completion_plan.md`, `progress_checkpoint.md`, `remaining_milestones.md` | the acceptance contract; the checkpoint after milestones E, F and M; the design and register work of milestones T (telemetry), C (administration and content), B (game ballistics), V (verification completeness) and S (staging) |
| Design notes | the other eleven | one subject each: the semantics chosen for a group of requirements, the lock order, the refusals with their codes, and the tests that hold them |

The design notes by domain:

| Domain | Notes |
|---|---|
| [identity and access](/documentation_v2/glossary.md#identity-and-access) | `identity_transactions.md` |
| [operations](/documentation_v2/glossary.md#operations) | `event_administration.md`, `event_eligibility_allocation.md`, `reservation_attendance.md`, `reservation_mutation_guards.md`, `reservation_transaction_design.md`, `live_occupancy.md` |
| [missions](/documentation_v2/glossary.md#missions) | `mission_artifacts.md` |
| [server infrastructure](/documentation_v2/glossary.md#server-infrastructure) | `machine_credentials.md`, `fleet_command_ledger.md` |
| verification | `property_test_evidence.md` |

Requirement identifiers start with their area: `events` (14), `identity` (13), `verification`
(10), `missions` (8), `fleet` (7), `telemetry` (7), `administration` (6), `content` (3),
`staging` (3) and `dashboard` (2).

### What the readiness check reads

`cargo xtask verify api-readiness` (`tools_v2/xtask/src/verifications/api_readiness/`) reads this
folder in two ways:

1. It reads and validates `requirements.json`, named by `API_READINESS_REGISTER` in
   `tools_v2/xtask/src/core/repository_layout.rs`, before it judges any receipt: unknown fields,
   duplicate identifiers, a missing implementation path, a check no requirement names or a check
   without its marker or pattern stop the run.
2. Its source fingerprint covers every file here with a source extension, Markdown included,
   through `API_READINESS_EVIDENCE_PREFIX`, together with the code trees. A receipt records the
   fingerprint it was produced under, so any edit in this folder, this README included, makes
   every recorded receipt stale until the next `--execute` run.

The receipts themselves are not here: `--execute` writes `<check id>.log` and `<check id>.json`
into `target/api-readiness` (or `--evidence <dir>`), and the operational checks' receipts come
from an external staging runner. The check exits 0 when every requirement holds, 1 when a receipt
is rejected or a requirement unmet, and 2 when a receipt is missing.

### Reading notes

The notes describe the code at the time each was written; where the code has moved since, the
code wins:

- `mission_artifacts.md:57` spells the artifact path parameter `{artifactId}`; the route is
  `/api/v1/missions/{id}/artifacts/{artifact_id}` (`apps/website/api_v2/src/missions/routes.rs`).
- `mission_artifacts.md:82-93` lists the deployment refusals without the codes
  `SERVER_INACTIVE` and `EVENT_MISSION_NOT_ON_SERVER`, which the code returns for an inactive
  server and for an event mission that does not fit the server
  (`apps/website/api_v2/src/missions/services/mission_deployments/deployment_requests.rs`; the
  [mission deployments README](/apps/website/api_v2/src/missions/services/mission_deployments/README.md)
  has the full table).
- `progress_checkpoint.md:7-8` says the work is uncommitted; it is committed on `main`.

## Code

- [API crate](/apps/website/api_v2/) — the domains the requirements and notes describe; each
  requirement's `implementation` entries name the exact paths.
- [API readiness check](/tools_v2/xtask/src/verifications/api_readiness/) — the verifier that
  validates the register, fingerprints this folder and judges the receipts.
- [Repository layout](/tools_v2/xtask/src/core/repository_layout.rs) — `API_READINESS_REGISTER`
  and `API_READINESS_EVIDENCE_PREFIX`, the two constants that name this folder.

## Boundaries

- Depends on: the API code and its test suites, which the checks run; the readiness check's
  register schema in `tools_v2/xtask/src/verifications/api_readiness/register.rs`.
- Used by: `cargo xtask verify api-readiness`; the READMEs of the API domains, workers and
  services, the glossary and the frontend administration docs, which link the design notes.
- Rules: the register stays valid for `register.rs` (the check refuses to run otherwise); a
  requirement names only checks that exist, and every check is named by a requirement; the files
  keep their names, since the register and the READMEs link them; an edit here invalidates the
  recorded evidence, so it lands with a fresh `--execute` run.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — the domains and routes the
  requirements cover.
- [API decisions](/documentation_v2/website/api_v2/decisions.md) — the cross-domain decisions the
  notes build on.
- [API readiness verification](/tools_v2/xtask/src/verifications/api_readiness/README.md) — how
  each receipt is judged.
