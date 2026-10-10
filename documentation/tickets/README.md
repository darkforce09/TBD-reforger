**Status:** live

# Ticket specs and plans

The documents the [tickets](/documentation/glossary/n_to_z.md#ticket) cite: each ticket's
spec, which says what to build and how it is accepted, and its plan, which says how the work
runs. Agents read them before working a ticket, and `ttm import` reads them into the central
ticket manager beside the legacy ticket files.

## Contents

```text
documentation/tickets/
├── plans/  one four-section plan per ticket that went ready
└── specs/  ticket specs, one flat folder
```

## How it works

A legacy ticket file `.ai/tickets/T-<id>.toml` names its documents in two fields, each a
repository-relative path: `spec` names a file in `specs/` and `plan` names a file in `plans/`.
Both folders are flat, and a file's name carries its ticket id:

| Document | Path | Written from |
|---|---|---|
| spec | `documentation/tickets/specs/t<id>_<subject>.md`, the id without `T-`, dots as underscores | `.ai/tickets/spec_template.md` |
| plan | `documentation/tickets/plans/t-<id>_plan.md`, the id lowercased, dots as underscores | `.ai/tickets/plan_template.md` |

A document follows its ticket's status:

| Ticket status | Spec and plan | Status line |
|---|---|---|
| `idea`, `queued`, `ready` | live: written and corrected as the design settles | `**Status:** live` |
| `shipped`, `cancelled` | frozen record: never reworded | `**Status:** frozen record` |

Once a document is frozen, only a link to a moved document changes, and a link to code that no
longer exists becomes a GitHub permalink. Knowledge that outlasts the ticket then moves into the
feature doc of the code it describes.

`ttm --project reforger mark-ready <ticket>` refuses to promote a ticket the project's readiness
gate does not admit, and `ttm --project reforger check` reports the tickets that break it; the
ticket manager holds each ticket's spec and plan once imported.

## Code

- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — `brief`, which
  returns the spec and plan to read from `ttm --project reforger brief`, and `show`, which says
  whether the ticket manager holds a ticket's spec.
- [Platform slice runs](/tools/commands/platform_execution/src/slice_execution.rs) — refuses a
  slice whose spec the ticket manager does not hold, and prompts the agent with its brief.

## Boundaries

- Depends on: the legacy ticket files in `.ai/tickets/`, whose `spec` and `plan` fields name
  these files; the templates `.ai/tickets/spec_template.md` and `.ai/tickets/plan_template.md`.
- Used by: `ttm import`, which reads them into the central ticket manager; `TICKET_DOCUMENTS_DIR`
  in `tools/foundation/repository_layout/src/documentation_locations.rs`, through which the
  documentation gates judge the tree as frozen records; feature docs and runbooks that link a
  spec.
- Rules: both folders stay flat, with no subfolders; a file keeps the name its ticket's field
  cites; a frozen record is never reworded; the tree is exempt from the 500-line limit and judged
  only on its links (`cargo xtask verify link-check`).

## Related documentation

- [Ticket identifiers](/documentation/standards/ticket_identifiers.md) — the id grammar and
  the spec and plan paths.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — the run, land and
  close around a ticket's spec and plan.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  document lifecycle and status lines.
- [Legacy ticket data](/.ai/tickets/README.md) — the ticket files, awaiting import, that cite
  these documents.
