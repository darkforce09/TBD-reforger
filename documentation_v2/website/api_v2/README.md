**Status:** live

# API documentation

The documentation of the website [API](/documentation_v2/glossary/a_to_f.md#api), the `website-api`
crate in `apps/website/api_v2/`: the cross-domain overview, the environment variable reference,
the decisions behind the design, and the verification evidence the API is accepted against.
Developers and AI agents read it before changing the API or deploying it.

## Contents

```text
documentation_v2/website/api_v2/
├── api_overview.md           the map of the API: boot, request path, callers, routes by domain, open work
├── decisions.md              the cross-domain design decisions, dated, with their consequences
├── environment_variables.md  every variable the API reads: default, requirement, failure, reader
└── verification_evidence/    the acceptance register, the program records and the design notes
```

## How it works

Start with [api_overview.md](/documentation_v2/website/api_v2/api_overview.md): it follows a
request from boot to a domain, says who calls which routes, lists the open tickets and leads to
each domain. The layers split the facts:

- The in-code READMEs under `apps/website/api_v2/` state what the code declares: each domain's
  routes with their tiers, its models and services, its rules and the tests that hold them. The
  documents here link them rather than repeat them.
- The documents here carry what spans domains: the overview, the
  [environment variable reference](/documentation_v2/website/api_v2/environment_variables.md),
  the [decisions log](/documentation_v2/website/api_v2/decisions.md) and the open work.
- `verification_evidence/` holds the per-domain design depth (transactions, lock orders, refusal
  codes) and the register `cargo xtask verify api-readiness` judges; its
  [README](/documentation_v2/website/api_v2/verification_evidence/README.md) indexes each file.

No domain has a document of its own here: each domain README and the design notes in
`verification_evidence/` cover it. A domain whose behaviour, open work or decisions outgrow them
gets a `<domain>.md` feature doc beside the overview, following the
[feature doc template](/documentation_v2/standards/templates/feature_doc.md), and a line in
Contents. A new cross-domain decision is a new entry at the end of `decisions.md`.

| Domain | Code README | Design notes |
|---|---|---|
| identity and access | [identity_and_access](/apps/website/api_v2/src/identity_and_access/README.md) | [identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md) |
| administration | [administration](/apps/website/api_v2/src/administration/README.md) | none |
| operations | [operations](/apps/website/api_v2/src/operations/README.md) | [event eligibility](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md), [event administration](/documentation_v2/website/api_v2/verification_evidence/event_administration.md), [live occupancy](/documentation_v2/website/api_v2/verification_evidence/live_occupancy.md), reservations (three notes) |
| missions | [missions](/apps/website/api_v2/src/missions/README.md) | [mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md) |
| match telemetry | [match_telemetry](/apps/website/api_v2/src/match_telemetry/README.md) | [reservation and attendance](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md) |
| command center | [command_center](/apps/website/api_v2/src/command_center/README.md) | none |
| community content | [community_content](/apps/website/api_v2/src/community_content/README.md) | none |
| server infrastructure | [server_infrastructure](/apps/website/api_v2/src/server_infrastructure/README.md) | [machine credentials](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md), [fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md) |

## Code

- [API crate](/apps/website/api_v2/) — the crate every document here describes; its
  [README](/apps/website/api_v2/README.md) is the atlas of its folders.
- [API core](/apps/website/api_v2/src/core/) — the router, middleware and configuration the
  overview and the environment variable reference describe.
- [Background workers](/apps/website/api_v2/src/background_workers/) — the interval tasks the
  overview summarises.

## Boundaries

- Depends on: the code of `apps/website/api_v2/`, which every claim is checked against; the
  [feature doc](/documentation_v2/standards/templates/feature_doc.md) and
  [decisions entry](/documentation_v2/standards/templates/decisions_entry.md) templates; the
  ticket registry in `.ai/tickets/` for open work.
- Used by: the READMEs of the API crate and its domains, which link the overview under Related
  documentation; the [website documentation](/documentation_v2/website/README.md) index; the
  glossary's evidence links; the frontend page docs that describe what their calls mean
  server-side.
- Rules: every route, variable and default is stated as the code has it, and a disagreement
  with `.env.example` or another document is recorded, not copied; decisions entries are never
  rewritten, and a changed decision is a new entry that supersedes the old one; the evidence
  files are indexed, never reworded.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running the API and its
  database locally, with the dev login and Discord sign-in.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — building and running
  the API on the deploy host.
- [Where does X go](/documentation_v2/standards/where_does_x_go.md) — where new code belongs in
  the crate.
