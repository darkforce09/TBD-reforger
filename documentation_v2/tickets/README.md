**Status:** live

# Ticket specs and plans

The documents the [ticket](/documentation_v2/glossary/n_to_z.md#ticket) registry cites: each ticket's
spec, which says what to build and how it is accepted, and its plan, which says how the work
runs. Agents read them before working a ticket, and the
[ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) and the ticket commands read their paths.

## Contents

```text
documentation_v2/tickets/
├── plans/  one four-section plan per ticket that went ready
└── specs/  ticket specs, one flat folder
```

## How it works

A ticket file `.ai/tickets/T-<id>.toml` names its documents in two fields, each a
repository-relative path: `spec` names a file in `specs/` and `plan` names a file in `plans/`.
Both folders are flat, and a file's name carries its ticket id:

| Document | Path | Written from |
|---|---|---|
| spec | `documentation_v2/tickets/specs/t<id>_<subject>.md`, the id without `T-`, dots as underscores | `.ai/tickets/spec_template.md` |
| plan | `documentation_v2/tickets/plans/t-<id>_plan.md`, the id lowercased, dots as underscores | `.ai/tickets/plan_template.md` |

A document follows its ticket's status:

| Ticket status | Spec and plan | Status line |
|---|---|---|
| `idea`, `queued`, `ready` | live: written and corrected as the design settles | `**Status:** live` |
| `shipped`, `cancelled` | frozen record: never reworded | `**Status:** frozen record` |

Once a document is frozen, only a link to a moved document changes, and a link to code that no
longer exists becomes a GitHub permalink. Knowledge that outlasts the ticket then moves into the
feature doc of the code it describes.

`cargo xtask ticket mark-ready <id> <spec> [plan]` refuses to promote a ticket while its spec or
plan file is missing, and defaults an unset `plan` to the id-derived path.
`cargo xtask ticket check` reports every `spec` or `plan` that names no file, for every status but
`idea` and `cancelled`.

## Code

- [Ticket engine](/tools_v2/ticket-engine/) — `SPECS_DIR`, `PLANS_DIR` and `plan_path` in
  `tools_v2/ticket-engine/src/repository.rs`; the existence and plan ready-gate checks in
  `tools_v2/ticket-engine/src/validation/`; `mark_ready` in
  `tools_v2/ticket-engine/src/ops/readiness.rs`.
- [Ticketboard](/apps/ticketboard/) — shows each ticket's `spec` and `plan` in its detail panel
  and opens a Markdown one in the in-app document viewer.
- [Ticket commands](/tools_v2/xtask/src/commands/ticket/) — `ticket mark-ready`, `ticket check`,
  `ticket brief` (prints the spec and plan to read) and `ticket prompt` (reads a spec's
  `## Claude Code prompt` block).

## Boundaries

- Depends on: the ticket registry in `.ai/tickets/`, whose `spec` and `plan` fields name these
  files; the templates `.ai/tickets/spec_template.md` and `.ai/tickets/plan_template.md`.
- Used by: the ticket engine and the `cargo xtask ticket` commands above; the ticketboard; the
  platform slice dispatch (`tools_v2/xtask/src/commands/platform/slice_execution.rs`), which reads
  a ticket's `spec`; `TICKET_DOCUMENTS_DIR` in `tools_v2/xtask/src/core/repository_layout.rs`,
  through which the documentation gates judge the tree as frozen records; feature docs and
  runbooks that link a spec.
- Rules: both folders stay flat, with no subfolders; a file keeps the name its ticket's field
  cites (`cargo xtask ticket check`); a frozen record is never reworded; the tree is exempt from
  the 500-line limit (`cargo xtask verify markdown-placement`) and judged only on its links
  (`cargo xtask verify link-check`).

## Related documentation

- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — the id grammar and
  the spec and plan paths.
- [Ticket run pipeline](/documentation_v2/runbooks/ticket_run_pipeline.md) — writing a spec and a
  plan and marking a ticket ready.
- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — the
  document lifecycle and status lines.
- [Ticket registry](/.ai/tickets/README.md) — the ticket files that cite these documents.
- [Ticketboard viewer](/documentation_v2/ticketboard/ticketboard_viewer.md) — the board's
  behaviour, including the Mark ready form that sets a ticket's spec.
