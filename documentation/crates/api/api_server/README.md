**Status:** live

# API documentation

The documentation of the website [API](/documentation/glossary/a_to_f.md#api), the API server crate
`api_server` in `crates/api/api_server/` and the API crates in `crates/api/` it is assembled from: the
cross-domain overview, the environment variable reference,
the decisions behind the design, and the verification evidence the API is accepted against.
Developers and AI agents read it before changing the API or deploying it.

## Contents

```text
documentation/crates/api/api_server/
├── api_overview.md           the map of the API: boot, request path, callers, routes by domain, crate map, state, ids, open work
├── decisions.md              the cross-domain design decisions, dated, with their consequences
├── environment_variables.md  every variable the API reads: default, requirement, failure, reader
└── verification_evidence/    the acceptance register, the program records and the design notes
```

## How it works

Start with [api_overview.md](/documentation/crates/api/api_server/api_overview.md): it follows a
request from boot to a domain, says who calls which routes, lists the open tickets and leads to
each domain. The layers split the facts:

- The in-code READMEs under `crates/api/api_server/` and `crates/api/` state what the code declares: each domain's
  routes with their tiers, its models and services, its rules and the tests that hold them. The
  documents here link them rather than repeat them.
- The documents here carry what spans domains: the overview, the
  [environment variable reference](/documentation/crates/api/api_server/environment_variables.md),
  the [decisions log](/documentation/crates/api/api_server/decisions.md) and the open work.
- `verification_evidence/` holds the per-domain design depth (transactions, lock orders, refusal
  codes) and the register `cargo xtask verify api-readiness` judges; its
  [README](/documentation/crates/api/api_server/verification_evidence/README.md) indexes each file.

No domain has a document of its own here: each domain README and the design notes in
`verification_evidence/` cover it. A domain whose behaviour, open work or decisions outgrow them
gets a `<domain>.md` feature doc beside the overview, following the
[feature doc template](/documentation/standards/templates/feature_doc.md), and a line in
Contents. A new cross-domain decision is a new entry at the end of `decisions.md`.

| Domain | Code README | Design notes |
|---|---|---|
| identity and access | [identity_and_access](/crates/api/api_identity_and_access/src/README.md) | [identity transactions](/documentation/crates/api/api_server/verification_evidence/identity_transactions.md) |
| administration | [administration](/crates/api/api_administration/src/README.md) | none |
| operations | [operations](/crates/api/api_operations/src/README.md) | [event eligibility](/documentation/crates/api/api_server/verification_evidence/event_eligibility_allocation.md), [event administration](/documentation/crates/api/api_server/verification_evidence/event_administration.md), [live occupancy](/documentation/crates/api/api_server/verification_evidence/live_occupancy.md), reservations (three notes) |
| missions | [missions](/crates/api/api_missions/src/README.md) | [mission artifacts](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md) |
| match telemetry | [match_telemetry](/crates/api/api_match_telemetry/src/README.md) | [reservation and attendance](/documentation/crates/api/api_server/verification_evidence/reservation_attendance.md) |
| command center | [command_center](/crates/api/api_command_center/src/README.md) | none |
| community content | [community_content](/crates/api/api_community_content/src/README.md) | none |
| server infrastructure | [server_infrastructure](/crates/api/api_server_infrastructure/src/README.md) | [machine credentials](/documentation/crates/api/api_server/verification_evidence/machine_credentials.md), [fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md) |

## Code

- [API server](/crates/api/api_server/) — the crate every document here describes, with the
  `api-server` and `import-item-registry` binaries; its [README](/crates/api/api_server/README.md)
  is the atlas of its folders.
- [API crates](/crates/api/README.md) — the infrastructure, kernel, domain and worker crates the
  API server is assembled from.
- [API source](/crates/api/api_server/src/README.md) — the router and the composition root the overview and
  the environment variable reference describe.
- [API HTTP layer](/crates/api/api_http_layer/) — the middleware, extractors, rate limiters and
  metrics the router mounts.
- [Background workers](/crates/api/api_background_workers/src/) — the interval tasks the
  overview summarises.

## Boundaries

- Depends on: the code of `crates/api/api_server/` and `crates/api/`, which every claim is checked against; the
  [feature doc](/documentation/standards/templates/feature_doc.md) and
  [decisions entry](/documentation/standards/templates/decisions_entry.md) templates; the
  ticket registry in `.ai/tickets/` for open work.
- Used by: the READMEs of the API server and its crates, which link the overview under Related
  documentation; the [API crate documentation](/documentation/crates/api/README.md) index; the
  glossary's evidence links; the frontend page docs that describe what their calls mean
  server-side.
- Rules: every route, variable and default is stated as the code has it, and a disagreement
  with `.env.example` or another document is recorded, not copied; decisions entries are never
  rewritten, and a changed decision is a new entry that supersedes the old one; the evidence
  files are indexed, never reworded.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the API and its
  database locally, with the dev login and Discord sign-in.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and running
  the API on the deploy host.
- [Where does X go](/documentation/standards/where_does_x_go.md) — where new code belongs in
  the crate.
