**Status:** live

# Ticket specs

The specifications [tickets](/documentation_v2/glossary/n_to_z.md#ticket) cite: what a ticket or a
program of tickets builds, its design and how it is accepted. An agent working a ticket reads its
spec as the source of truth for the work.

## Contents

```text
documentation_v2/tickets/specs/
└── t*.md  one spec per file, named t<id>_<subject>.md after the ticket it was written for
```

## How it works

The folder is flat and holds only ticket specs. A spec's name is `t`, its ticket's id without
`T-` and with dots as underscores, then a snake_case subject: the spec of ticket `T-<n>.<m>` is
`t<n>_<m>_<subject>.md`. A program's spec and the specs of its children share the program's id as
a prefix, so they sort together.

A ticket cites a spec in one of two ways, both as a repository-relative path:

- its `spec` field in `.ai/tickets/T-<id>.toml`, the spec its work is accepted against;
- a `Design: <path>.` line in its `citations` field, for a design document the ticket draws on.

A new spec is written from `.ai/tickets/spec_template.md`, starts with `**Status:** live`, and
stays live while its ticket is `idea`, `queued` or `ready`. When the ticket ships or is cancelled
the spec becomes a frozen record: its status line reads `**Status:** frozen record` and its text is
never reworded again, only its links to moved documents. A spec meant for
`cargo xtask ticket prompt` holds a `## Claude Code prompt` heading followed by a fenced block,
which the command prints.

## Code

- [Ticket engine](/tools_v2/ticket-engine/) — `SPECS_DIR` in
  `tools_v2/ticket-engine/src/repository.rs` names this folder; the existence check in
  `tools_v2/ticket-engine/src/validation/references.rs` reports a `spec` that names no file;
  `mark_ready` in `tools_v2/ticket-engine/src/ops/readiness.rs` refuses a missing spec; the
  prompt extractor is `tools_v2/ticket-engine/src/cli/prompt.rs`.
- [Ticketboard](/apps/ticketboard/) — shows a ticket's `spec` and opens it in the in-app
  document viewer.

## Boundaries

- Depends on: the `spec` and `citations` fields of the ticket files in `.ai/tickets/`; the spec
  template `.ai/tickets/spec_template.md`.
- Used by: `cargo xtask ticket check`, `ticket mark-ready`, `ticket brief` and `ticket prompt`; the
  ticketboard; the platform slice dispatch
  (`tools_v2/xtask/src/commands/platform/slice_execution.rs`); runbooks and feature docs that link
  the spec of a program, such as the [mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md).
- Rules: no subfolders and no document that is not a ticket spec; a spec keeps the path its
  tickets cite, which `cargo xtask ticket check` verifies for every ticket not `idea` or
  `cancelled`; a frozen spec is never reworded, and the gates judge it only on its links
  (`cargo xtask verify link-check`).

## Related documentation

- [Ticket specs and plans](/documentation_v2/tickets/README.md) — the lifecycle both folders share.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — the id grammar behind
  the file names.
- [Ticket run pipeline](/documentation_v2/runbooks/ticket_run_pipeline.md) — where the spec is
  written in a ticket's life.
