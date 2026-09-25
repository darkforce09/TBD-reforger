**Status:** live

# Audit records

Whole-repository audits of the codebase and the website's architecture, with the verification of
their claims and the maps from each finding to the [ticket](/documentation_v2/glossary.md#ticket)
that fixed it. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/audits/
├── audit_2026_09_false_claims.md           claims of the September audit the code proved false
├── audit_2026_09_finding_to_ticket_map.md  each verified finding of that audit and its ticket
├── codebase_audit_2026.md                  frontend, backend and mod audit, fixed in one bundle
├── fable_5_audit_program.md                order and outcome of an audit remediation program
├── frontend_data_provenance.md             render sites fed by the API, and those on mock data
└── website_architecture_audit.md           website architecture and performance audit
```

## How it works

Each audit names its date, the commit it read and the tickets its findings became; the September
pair splits one audit into the findings that held, each with its ticket, and the claims the code
disproved, each with the code that disproves it. The findings are closed or tracked as tickets in
`.ai/tickets/`, so an audit is read for the reasoning behind a ticket, not as a list of open
defects. Paths and line counts are as they stood when each audit was written.

## Code

- [Website](/apps/website/) — the frontend, the API and the engine crates the audits read.
- [Mod](/apps/mod/) — the addons the codebase audit also covered.

## Boundaries

- Depends on: nothing live; the audits quote the code of their time.
- Used by: the ticket files in `.ai/tickets/` whose citations name an audit as their source; the
  [cold start and preflight](/documentation_v2/runbooks/factory_waves/cold_start_and_preflight.md)
  factory runbook, which cites the frontend data provenance audit; the Cursor rule
  `.cursor/rules/cursor-agent-workflow.mdc`, which cites the codebase audit.
- Rules: never reworded, only links change; a new audit lands here only once its findings are
  filed as tickets or closed.

## Related documentation

- [Known bugs](/documentation_v2/known_bugs/README.md) — the live registry of reproduced defects.
- [Ticket registry](/.ai/tickets/README.md) — where each audit finding is tracked.
