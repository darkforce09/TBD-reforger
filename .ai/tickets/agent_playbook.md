# Ticket agent playbook

The recipes an agent, or a person, follows for each [ticket](/documentation_v2/glossary/n_to_z.md#ticket)
task: filing, filling, queuing, writing the spec and plan, running, shipping, cancelling. The
source of truth is one `.ai/tickets/T-<id>.toml` file per ticket; the
[ticket registry README](/.ai/tickets/README.md) describes the folder, and
[Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) gives
every step with its expected output.

## Golden rule

Change a ticket with a `cargo xtask ticket` verb, or by a hand edit of its body fields in the
engine's canonical form → run `cargo xtask ticket check --strict` → commit the ticket file together
with the derived files the verb wrote and the code and documentation the work changed.

- A ticket file is created only by `cargo xtask ticket add` or `ticket add-child`.
- Never edit by hand what `cargo xtask ticket sync` writes: `.ai/tickets/queue.json` and the
  next-work block between `<!-- ticket-sync:next:start -->` and `<!-- ticket-sync:next:end -->` in
  [the Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md).
  The ticket column of the
  [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
  is kept by hand.
- Never edit `.ai/tickets/wave.lock`; `cargo xtask wave repack` writes it.

## HARD — no deferrals without the operator's word

Do not put "fold forward", "deferred to a later slice" or a self-authored out-of-scope list into a
spec, plan, handoff or prompt unless the **operator explicitly** authorized that deferral in the
conversation. A complete-outcome ask ("replace the whole surface", "finish the port") means
**finish**, not a thin first pass plus an appendix. Rule:
[`no-silent-deferrals.mdc`](/.cursor/rules/no-silent-deferrals.mdc).

## Status lifecycle

| You want to | Do |
|---|---|
| file an idea | `cargo xtask ticket add "<title>" --summary "<the problem, with file:line>"` → status `idea`, no order |
| file a slice of a program | `cargo xtask ticket add-child <parent> "<title>"` |
| move it to the backlog | `cargo xtask ticket reorder <id> <id to place it after>` → `queued`, with an order |
| make it runnable | fill the body, write the spec and the plan, then `cargo xtask ticket mark-ready <id> <spec>` → `ready` |
| mark it in progress or in review | `cargo xtask ticket set-status <id> running` or `review` |
| finish a program's slice | `cargo xtask ticket advance-slice <program id>` |
| ship it | `cargo xtask ticket ship <id>`, commit, then `cargo xtask ticket stamp-sha <id> <sha>` |
| put it off (operator's word only) | `cargo xtask ticket set-status <id> deferred` |
| drop it | `cargo xtask ticket set-status <id> cancelled` — the file stays |
| reorder | `cargo xtask ticket reorder <id> <id to place it after>` |

## Recipes

### File and fill a ticket

1. `cargo xtask ticket add "<title>" --summary "<the problem, with file:line>"`; check the
   `class` and `[scope]` it guessed.
2. Fill `main_goal`, `context`, `requirement`, `current_state`, `approach`, `verify` and
   `acceptance`, and `executor` when the work is not for an AI coding agent, in canonical form.
3. `cargo xtask ticket check --strict`.

### Write the spec and plan, and mark it ready

1. Spec: `documentation_v2/tickets/specs/t<id>_<subject>.md` from
   [`spec_template.md`](/.ai/tickets/spec_template.md). When the spec should feed
   `cargo xtask ticket prompt`, add its prompt block per
   [`implementation_prompt.md`](/.ai/tickets/implementation_prompt.md).
2. Plan: copy [`plan_template.md`](/.ai/tickets/plan_template.md) to
   `documentation_v2/tickets/plans/t-<id, lowercased, dots as underscores>_plan.md` and fill its
   four sections.
3. Handoff, only when the operator's context does not fit in the spec:
   `.ai/artifacts/<slug>_claude_code_handoff.md` from
   [`handoff_template.md`](/.ai/tickets/handoff_template.md).
4. `cargo xtask ticket mark-ready <id> <spec path>` — it refuses while the spec or plan is missing,
   the ticket has no order, a body field is empty or a `depends_on` ticket is still open.

### Run it

```bash
cargo xtask ticket run --dry-run
cargo xtask ticket run
```

`ticket run` takes up to 10 `ready` tickets with a spec and the `claude-code` executor and hands
each to `cargo xtask platform slice-run <id>`, which starts the agent command and writes a run
receipt. Preview first: the queue may hold other ready tickets ahead of yours. To run one ticket
alone, use `cargo xtask platform slice-run <id>`. To give the work to an agent in a chat instead,
print its prompt with `cargo xtask ticket prompt <id>` and deliver it as one fenced block
([`claude-prompt-delivery.mdc`](/.cursor/rules/claude-prompt-delivery.mdc)).

### Ship it

1. Verify against the ticket's own `verify` lines (`cargo xtask ticket get <id> verify`) and the
   checks the [commit checklist](/documentation_v2/standards/commit_checklist.md) lists.
2. `cargo xtask ticket ship <id>`.
3. Commit the code, the documentation it changes and the ticket files together, with the ticket id
   in the subject: documentation ships in the same commit as the code it describes. Update the
   feature doc's `## Open work`.
4. `cargo xtask ticket stamp-sha <id> $(git rev-parse --short HEAD)`, then commit the ticket file
   and its estimate.

### Brainstorm

1. File each idea with `cargo xtask ticket add`.
2. Review the `idea` and `deferred` columns in the [ticketboard](/apps/ticketboard/README.md).
3. When one is promoted: reorder it, write its spec and plan, mark it ready.

### Read a ticket

```bash
cargo xtask ticket show <id>
cargo xtask ticket brief <id>
cargo xtask ticket next
```

## Executor gate

`cargo xtask ticket run` runs only tickets whose executor (a program's: its active slice's) is
`claude-code`; it never runs the others.

| Executor | Taken by | Work |
|---|---|---|
| `claude-code` | any AI coding agent, through the ticket tooling or a chat prompt | code, with its tests and documentation |
| `cursor-docs` | an agent in a chat | a ticket, spec or documentation pass |
| `workbench` / `human` | a person | Workbench and hands-on work in `apps/mod/`; filter the [ticketboard](/apps/ticketboard/README.md) by executor |
| `ci` | CI | a CI lane |

An agent stops at a `workbench`, `human` or `ci` ticket and waits for that party.

## Cursor chats

A Cursor chat infers its mode from the message — plan review, ticket and documentation pass,
code, or platform factory — as
[`cursor-agent-workflow.mdc`](/.cursor/rules/cursor-agent-workflow.mdc) describes. By default it
writes one ticket at a time, and application code only on the operator's explicit word or through
slice agents in factory mode.

## Validation

```bash
cargo xtask ticket check --strict
```

`check` validates every ticket file against `.ai/tickets/schema.json` and the structural rules
and checks the wave lock; `--strict` adds the retired id-spelling scan, the check that the Eden gap
analysis carries no priority column, and the token counters. It prints `check OK`.
