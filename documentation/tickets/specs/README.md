**Status:** live

# Ticket specs

The specifications [tickets](/documentation/glossary/n_to_z.md#ticket) cite: what a ticket or a
program of tickets builds, its design and how it is accepted. An agent working a ticket reads its
spec as the source of truth for the work.

## Contents

```text
documentation/tickets/specs/
└── t*.md  one spec per file, named t<id>_<subject>.md after the ticket it was written for
```

## How it works

The folder is flat and holds only ticket specs. A spec's name is `t`, its ticket's id without
`T-` and with dots as underscores, then a snake_case subject: the spec of ticket `T-<n>.<m>` is
`t<n>_<m>_<subject>.md`. A program's spec and the specs of its children share the program's id as
a prefix, so they sort together.

A ticket cites a spec in one of two ways, both as a repository-relative path:

- its `spec` field in the legacy file `.ai/tickets/T-<id>.toml`, the spec its work is accepted
  against, which `ttm import` reads into the central ticket manager;
- a `Design: <path>.` line in its `citations` field, for a design document the ticket draws on.

A new spec is written from `.ai/tickets/spec_template.md`, starts with `**Status:** live`, and
stays live while its ticket is `idea`, `queued` or `ready`. When the ticket ships or is cancelled
the spec becomes a frozen record: its status line reads `**Status:** frozen record` and its text is
never reworded again, only its links to moved documents. A spec meant for
`ttm --project reforger prompt` holds a `## Claude Code prompt` heading followed by a fenced block,
which the command prints.

## Code

- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — `show`, which
  says whether the ticket manager holds a ticket's spec, and `brief`, which returns it.
- [Platform slice runs](/tools/commands/platform_execution/src/slice_execution.rs) — refuses a
  slice whose spec the ticket manager does not hold.

## Boundaries

- Depends on: the `spec` and `citations` fields of the legacy ticket files in `.ai/tickets/`;
  the spec template `.ai/tickets/spec_template.md`.
- Used by: `ttm import`, which reads each spec into the central ticket manager;
  `ttm --project reforger brief` and `prompt`; the platform slice run
  (`tools/commands/platform_execution/src/slice_execution.rs`); runbooks and feature docs that link
  the spec of a program, such as the [mod slice workflow](/documentation/runbooks/mod_slice_workflow.md).
- Rules: no subfolders and no document that is not a ticket spec; a spec keeps the path its
  tickets cite; a frozen spec is never reworded, and the gates judge it only on its links
  (`cargo xtask verify link-check`).

## Related documentation

- [Ticket specs and plans](/documentation/tickets/README.md) — the lifecycle both folders share.
- [Ticket identifiers](/documentation/standards/ticket_identifiers.md) — the id grammar behind
  the file names.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — where the spec is
  read in a ticket's life.
