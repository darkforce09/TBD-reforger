# Ticket registry

Every [ticket](/documentation_v2/glossary/n_to_z.md#ticket) of the project as one TOML file, the
schemas and derived files around them, and the templates and instructions agents follow to write a
ticket's spec, plan, handoff and prompt. The `ticket-engine` crate reads and writes the folder
through `cargo xtask ticket`; the [ticketboard](/apps/ticketboard/README.md) shows it.

## Contents

```text
.ai/tickets/
├── ROOT                      the marker file that makes a folder a checkout root for every ticket command
├── T-*.toml                  one ticket per file, parents and dotted children alike
├── agent_playbook.md         the recipes an agent follows for each ticket task
├── corpus-pins.toml          corpus facts no ticket states: ids never minted, the mod program, gap rows
├── estimates/                one token estimate per ticket, `T-<id>.json`, written by `ticket stamp-sha`
├── estimates.schema.json     the schema every token estimate satisfies
├── handoff_template.md       the skeleton of a long-form handoff for one slice
├── implementation_prompt.md  the standard for the copy-paste prompt a spec hands a coding agent
├── metrics.schema.json       the schema every run receipt satisfies
├── plan_template.md          the four-section plan every ticket needs before it goes ready
├── queue.json                the dispatch queue `ticket sync` writes: `ready`, `running` and `review` tickets with a spec, in order
├── schema.json               the schema every ticket file satisfies
├── scope-vocab.toml          the domain, layer, component and surface words a `[scope]` table uses
├── spec_template.md          the skeleton of a ticket spec
└── wave.lock                 the wave plan `cargo xtask wave repack` compiles from the tickets
```

## How it works

**The ticket files are the source of truth.** `.ai/tickets/T-<id>.toml` holds one ticket: its
`kind` (`program` with child [slices](/documentation_v2/glossary/n_to_z.md#slice), or `work`), `status`, `order`, `spec`, `plan`, `executor`,
body fields and `[scope]` table, validated against `schema.json` and `scope-vocab.toml`. The ticket
engine renders every file in one canonical form (`TicketFile` in
`tools_v2/ticket-engine/src/encoding.rs`), and every command that writes a ticket renders the
whole file again.

- A ticket file is created only by `cargo xtask ticket add` or `ticket add-child`, which mint the
  next id; nobody picks an id or writes a new file by hand.
- `status`, `order`, `shipped_at` and `completed_at` change only through the verbs (`reorder`,
  `set-status`, `mark-ready`, `ship`, `stamp-sha`), which check what they write.
- A hand edit of the body fields keeps the canonical key order and adds no key the schema lacks;
  [Editing a ticket file by hand](/documentation_v2/runbooks/ticket_run_pipeline.md#editing-a-ticket-file-by-hand)
  gives the order.
- The verbs that write a ticket file (`add`, `add-child`, `remove`, `reorder`, `set-status`,
  `mark-ready`, `advance-slice`, `ship`, and `done` through `ship`) refuse, and write nothing,
  while `cargo xtask ticket check` is red; `ship --no-repack` waives only the findings whose fix is
  a repack. `stamp-sha` runs no check, because it is the step that turns the window between `ship`
  and the landing commit, red by design, green again; `ticket sync` writes its derived files
  without a check.

**Derived files are never edited by hand.** `cargo xtask ticket sync` regenerates `queue.json`
and the next-work block of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
roadmap, between `<!-- ticket-sync:next:start -->` and `<!-- ticket-sync:next:end -->` in
`documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md`. The ticket column of
the Eden gap analysis is kept by hand: sync's column writer rewrites only a table headed with a
`priority` column, which that table does not have. `wave.lock` is written only by
`cargo xtask wave repack` (which `ticket ship` and `ticket set-status` run). The run receipts under
`metrics/<id>/` are written by `cargo xtask platform slice-run`, and the folder exists only once
the first receipt does. No other file is generated from the tickets.

**Statuses.**

| Status | Meaning |
|---|---|
| `idea` | filed, with no order |
| `queued` | in the backlog, with an order |
| `ready` | spec and plan written; `ticket run` may take it |
| `running` | an agent is working on it (set by hand) |
| `review` | the work is waiting for a person to verify it (set by hand) |
| `shipped` | landed; `shipped_at` names the landing commit |
| `deferred` | put off by the operator; still open work |
| `cancelled` | dropped; the file stays |

**Executors.** A ticket's `executor` says who may take it: `claude-code` means any AI coding agent
run through the ticket tooling, whoever makes it (a ticket without one counts as `claude-code`);
`cursor-docs` a ticket, spec or documentation pass; `workbench`, `human` and `ci` mean an agent
stops and waits for that party. `ticket run` runs `claude-code` tickets only.

**Specs, plans and templates.** A ticket's spec is
`documentation_v2/tickets/specs/t<id>_<subject>.md`, written from `spec_template.md`, and its plan
is `documentation_v2/tickets/plans/t-<id>_plan.md`, copied from `plan_template.md`; `mark-ready`
refuses while either is missing. Both are live while the ticket is `idea`, `queued` or `ready`, and
become frozen records once it ships or is cancelled; the knowledge that outlasts the ticket then
moves into the feature doc of the code it describes, whose `## Open work` section links the open
tickets. A spec that feeds `cargo xtask ticket prompt` holds a prompt block built to
`implementation_prompt.md`; a handoff (`.ai/artifacts/<slug>_claude_code_handoff.md`, from
`handoff_template.md`) is written only when the context does not fit in the spec.
`agent_playbook.md` gives the recipe for each task.

```text
ticket add ─▶ fill the body ─▶ reorder (idea → queued) ─▶ spec + plan ─▶ mark-ready (→ ready)
  ─▶ ticket run ─▶ verify ─▶ ticket ship (→ shipped) ─▶ commit ─▶ ticket stamp-sha ─▶ commit
```

[Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) runs
that lifecycle step by step; [Factory waves](/documentation_v2/runbooks/factory_waves/README.md)
runs many tickets at once.

## Commands

Every command is `cargo xtask ticket <verb>`, run from the repository root; the
[ticket command group README](/tools_v2/xtask/src/commands/ticket/README.md) gives every flag and
exit code.

| Command | What it does |
|---|---|
| `cargo xtask ticket add "<title>" --summary "<text>"` | files a new work ticket with status `idea` |
| `cargo xtask ticket add-child <parent> "<title>"` | files the next dotted child of a program (`--promote` turns a work parent into a program) |
| `cargo xtask ticket reorder <id> <after>` | places a ticket after another; an `idea` becomes `queued` |
| `cargo xtask ticket mark-ready <id> <spec> [plan]` | sets the spec and plan and moves the ticket to `ready`, then syncs |
| `cargo xtask ticket set-status <id> <status>` | sets any of the eight statuses |
| `cargo xtask ticket advance-slice <id>` | moves a program's active slice to its next child |
| `cargo xtask ticket run [--dry-run]` | runs up to `batch_size` ready `claude-code` tickets through `platform slice-run` |
| `cargo xtask ticket ship <id>` | marks a ticket shipped and stamps `completed_at`, then syncs and repacks |
| `cargo xtask ticket stamp-sha <id> <sha>` | writes the landing commit to `shipped_at`, after the commit |
| `cargo xtask ticket check [--strict]` | validates every ticket file and the wave lock; prints `check OK` |
| `cargo xtask ticket sync` | regenerates `queue.json` and the roadmap's next-work block |
| `cargo xtask ticket show <id>` | prints one ticket's card |
| `cargo xtask ticket brief <id>` | prints what an executor reads first: spec, plan, body, acceptance |
| `cargo xtask ticket prompt <id> [--slice <slice id>] [--header]` | prints the prompt block of the (slice) spec |
| `cargo xtask ticket next` | shows the active slice and the next open tickets |
| `cargo xtask ticket list` | prints the queue in `queue.json` |
| `cargo xtask ticket metrics` | lists the run receipts |

`cargo xtask ticket --help` lists the other reading verbs (`get`, `plan-batch`, `ready-ids`,
`config`, `sparse-paths`, `scope-histogram`, `gap-round-trip`) and `remove`, `done` and `clean`.

## Boundaries

- Depends on: the `ticket-engine` crate (`tools_v2/ticket-engine/`), which owns the storage,
  validation, sync and wave packing; the xtask `ticket`, `wave` and `platform` command groups.
- Used by: `cargo xtask ticket`, `cargo xtask wave`, `cargo xtask slice-collisions` and the
  platform and mod wave drivers; the ticketboard; the `language-gates` job of
  `.github/workflows/ci.yml` (`ticket check --strict`); agents and people.
- Rules: the ticket files, `queue.json` and `wave.lock` change only through the commands above,
  apart from hand edits of body fields in canonical form; `cargo xtask ticket check --strict` passes
  after every change; `cargo test -p ticket-engine corpus_roundtrip_real_tree_byte_identical` proves
  every file is in canonical form.

## Related documentation

- [Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) — the
  lifecycle, `ticket run` and the run receipts.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — ids, fields, spec and
  plan names, ids in commit subjects.
- [Ticket specs and plans](/documentation_v2/tickets/README.md) — the spec and plan folders and
  their lifecycle.
- [Commit checklist](/documentation_v2/standards/commit_checklist.md) — what a landing commit
  carries.
- [Ticket engine](/tools_v2/ticket-engine/README.md) — the crate behind the registry.
