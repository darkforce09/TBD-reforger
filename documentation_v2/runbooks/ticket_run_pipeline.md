**Status:** live

# Taking a ticket from idea to shipped

Moves one [ticket](/documentation_v2/glossary/n_to_z.md#ticket) through its whole life: file it, fill it,
queue it, give it a spec and a plan, mark it ready, run an AI coding agent on it with
`cargo xtask ticket run`, then ship it and stamp its landing commit. Use it for tickets taken one
at a time; tickets that run in parallel go through a [wave](/documentation_v2/glossary/n_to_z.md#wave)
instead, as [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) describes. The
commands take seconds; the agent run takes as long as the work.

```text
ticket add ─▶ fill the body ─▶ reorder (idea → queued) ─▶ spec + plan ─▶ mark-ready (→ ready)
  ─▶ ticket run --dry-run ─▶ ticket run ─▶ platform slice-run, one ticket at a time ─▶ run receipt
  ─▶ verify ─▶ ticket ship (→ shipped) ─▶ commit ─▶ ticket stamp-sha ─▶ commit the stamp
```

## Prerequisites

- A checkout of `main` whose ticket files pass: `cargo xtask ticket check` prints `check OK`.
  Every command that writes a ticket refuses while the check is red, and writes nothing.
- For the run: the agent command on `PATH`. `platform slice-run` runs `TBD_SLICE_RUN_AGENT_CMD`
  when it is set (split on whitespace, the prompt appended as the last argument), else
  `claude --print --output-format json`; a Cursor agent is `agent --output-format json -p`. The
  agent must print a JSON answer that carries a usage object.
- The ticket rules this runbook relies on and does not repeat: the ticket fields, the spec and plan
  file names and the templates in [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md),
  and what a commit carries in the [commit checklist](/documentation_v2/standards/commit_checklist.md).

## How `ticket run` picks and runs tickets

`cargo xtask ticket run` reads `.ai/tickets/queue.json`, which `ticket sync` writes after every
ticket change. The queue holds the parent tickets whose status is `ready`, `running` or `review`
and that name a spec, sorted by `order` and then by id; `run` takes from it, in that order:

- only tickets whose status is `ready` and whose spec field is not empty;
- only tickets whose executor is `claude-code`. For a program the executor and the spec are those
  of its active slice. A ticket with no executor counts as `claude-code`, and the value means any AI
  coding agent run through the agent command, whoever makes it. `cursor-docs`, `workbench`,
  `human` and `ci` tickets are never run;
- with `--stream <S>`, only tickets of that stream;
- at most `batch_size` tickets, 10 by default.

Each chosen ticket goes to `cargo xtask platform slice-run <id>`, one after another; the
`concurrency=3` the command prints is not used. A ticket whose spec file is missing on disk is
skipped with `[<id>] SKIP — spec missing: <spec>`. The first failing run stops the batch: later
tickets are not run, and the command exits 1. When nothing qualifies, it prints
`No ready tickets. Steps:` with a hint and exits 1.

`platform slice-run` resolves the ticket, or a program's active slice, and refuses unless the
executor is `claude-code` and the spec exists. It then starts the agent with a fixed prompt: it
names the ticket and its spec, tells the agent to read `CLAUDE.md` and follow
`cargo xtask ticket brief <id>`, and to commit on the slice branch. The agent works in
`.ai/artifacts/worktrees/<id>` when that worktree exists, and in the main checkout otherwise. When
the agent exits 0 with a usage object, `slice-run` writes one run receipt:

```text
.ai/tickets/metrics/<id>/<started, without - and :>-<first 12 characters of HEAD>.json
```

A second receipt within the same second gets `-1`, `-2` and so on. The receipt records the id, the
agent (the basename of the program), `started`, `finished`, `outcome = "ran"`, the commit sha and
the token counts; `.ai/tickets/metrics.schema.json` is its schema. An agent that fails, prints no
JSON, or answers without a usage object fails the run and writes no receipt, never a zero count.
The id is always the slice-level id, the one `platform wave land` stamps. The folder does not exist
until the first receipt is written.

## Steps

Run every command from the repository root.

1. File the ticket.

   ```bash
   cargo xtask ticket add "<title>" --summary "<the problem, with file:line>"
   ```

   Expected: `Added T-<n>: <title>`; the new file `.ai/tickets/T-<n>.toml` is a work ticket with
   status `idea`, a `created_at` stamp and the scope `repo`/`docs`. `add` accepts `--program`,
   `--surfaces` and `--impact` and ignores them. It sets `class` from whole words of the title and
   summary, taking the first match in this order: `bug` (fix, bug, regression), `audit`, `docs`
   (doc, docs, readme, documentation), `chore` (refactor, cleanup, delete, port, rename, migrate,
   gate, ci), else `feature`. Check the class and scope in the file. A child of a program is
   `cargo xtask ticket add-child <parent> "<title>"`.

2. Fill the ticket body by hand: `main_goal`, `context`, `requirement`, `current_state`,
   `approach`, `verify` and `acceptance`, the right `class` and `scope`, and `executor` when the
   work is not for an AI coding agent. Keep the file in its canonical form (see
   [Editing a ticket file by hand](#editing-a-ticket-file-by-hand)), then check it.

   ```bash
   cargo xtask ticket check
   ```

   Expected: the debt counters, then `check OK`.

3. Queue the ticket: place it after another ticket in the queue order.

   ```bash
   cargo xtask ticket reorder <ticket id> <ticket id to place it after>
   ```

   Expected: `<ticket id> order -> <n> (after <other id>)`. An `idea` becomes `queued`; readiness
   needs an order, so an `idea` cannot go ready without this step.

4. Write the spec and the plan: the spec in `documentation_v2/tickets/specs/`, from
   `.ai/tickets/SPEC_TEMPLATE.md`, and the plan at the ticket's own plan path, from the plan
   template. A spec meant for `cargo xtask ticket prompt` holds a `## Claude Code prompt` heading
   followed by a fenced block; `ticket run` does not read it.

   ```bash
   cp .ai/tickets/plan_template.md documentation_v2/tickets/plans/t-<id, lowercased, dots as underscores>_plan.md
   ```

   Expected: no output; fill the four sections of the copy.

5. Mark the ticket ready.

   ```bash
   cargo xtask ticket mark-ready <ticket id> <spec path>
   ```

   Expected: `<ticket id> -> ready (<spec>; plan <plan>)`, then `ticket sync` rewrites
   `.ai/tickets/queue.json`. The plan argument is optional and defaults to the plan path of step 4.
   It refuses, and writes nothing, while the spec or the plan file is missing, while a
   `depends_on` ticket is neither `shipped` nor `cancelled`, while the ticket has no order, or while
   one of `context`, `requirement`, `current_state`, `approach`, `verify` or `acceptance` is empty.

6. Preview the batch. A real run starts agents at once, and the queue may hold other ready
   tickets ahead of yours.

   ```bash
   cargo xtask ticket run --dry-run
   ```

   Expected: `Running <k> ticket(s), concurrency=3 (dry_run=1)`, one
   `[<id>] branch=ticket/<id> spec=<spec> dry_run=true` line per ticket, then
   `Batch run finished. cargo run -q -p xtask -- ticket list`. The branch printed is only a label:
   nothing creates it. To run your ticket alone, skip step 7 and run
   `cargo xtask platform slice-run <ticket id>`, after checking it with `--dry-run`.

7. Run the batch.

   ```bash
   cargo xtask ticket run
   ```

   Expected: per ticket, `[<id>] slice-run agent=<program> spec=<spec> cwd=<directory>`, the agent's
   work, then `[<id>] receipt .ai/tickets/metrics/<id>/<file>.json`. To give the agent a slice
   branch instead of `main`, create its worktree first with
   `cargo xtask platform slice-worktree -- new <ticket id>`; the slice then lands through
   [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) steps 5 to 7,
   whose `land` stamps the receipt `landed` and refuses a slice without one unless given
   `--bookkeeping`.

8. Verify the work against the ticket's own `verify` field, and review the diff.

   ```bash
   cargo xtask ticket get <ticket id> verify
   ```

   Expected: the ticket's `verify` lines as one JSON array; run each command they name, and the
   checks the
   [commit checklist](/documentation_v2/standards/commit_checklist.md#verify-before-committing)
   lists for what the change touches. Nothing moves a ticket to `running` or `review` on its own;
   set either by hand with `cargo xtask ticket set-status <ticket id> review` when the state helps.

9. Ship the ticket.

   ```bash
   cargo xtask ticket ship <ticket id>
   ```

   Expected: `<ticket id> -> shipped`, after `ticket sync` and a repack of `.ai/tickets/wave.lock`;
   `completed_at` is stamped. It refuses while `ticket check` is red, and while `created_at`,
   `main_goal` or one of the six body fields of step 2 is empty. For a slice of a program, then
   move the program on with `cargo xtask ticket advance-slice <program id>`
   (`<program id> active_slice -> <next slice>`).

10. Commit the code, the documentation it changes and the ticket files, with the ticket id in the
    subject, as the [commit checklist](/documentation_v2/standards/commit_checklist.md) lists. This
    commit is the ticket's landing commit. Between the ship and the stamp, `ticket check` is red by
    design: a shipped ticket needs a `shipped_at`, which cannot exist before its commit.

    ```bash
    git rev-parse --short HEAD
    ```

    Expected: the landing commit's short sha.

11. Stamp the landing commit on the ticket.

    ```bash
    cargo xtask ticket stamp-sha <ticket id> <landing sha>
    ```

    Expected: `<ticket id>: shipped_at -> "<sha>"`, then either
    `<ticket id>: measured receipt(s) under .ai/tickets/metrics/<ticket id>/ — no estimate generated`
    or a `diff_loc` or `cohort_median` estimate written to `.ai/tickets/estimates/<ticket id>.json`
    with `"tokens" appended to estimated[]`. The same sha again changes nothing; another sha is
    refused. Commit the ticket file and the estimate by path. A wave ships and stamps each ticket
    before shipping the next ([Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md)
    step 12).

`cargo xtask ticket done <ticket id>` is `ticket clean` followed by step 9. `clean` removes a
worktree `.ai/artifacts/worktrees/TBD-<id>` and a branch `ticket/<id>` when they exist; nothing in
this pipeline creates either, and `done` never stamps, so step 11 still follows.

## Editing a ticket file by hand

A ticket file is TOML that the ticket engine renders in one canonical form, and every command that
writes a ticket renders the whole file again. Keep a hand edit in that form, or the next command
rewrites it and the diff grows:

- keys in the order the engine writes them: `id`, `kind`, `title`, `summary`, `class`, `status`,
  `order`, `spec`, `plan`, `executor`, `notes`, `priority`, `depends_on`, `unblocks`, `parent`,
  `children`, `active`, `main_goal`, `context`, `requirement`, `current_state`, `approach`,
  `verify`, `acceptance`, `citations`, `shipped_at`, `created_at`, `completed_at`, `estimated`,
  `estimate_note`, `migration_legacy`, `owns`, `pack_last`, then the `[scope]` table last
  (`TicketFile` in `tools_v2/ticket-engine/src/encoding.rs`);
- no key the schema does not define: a new key goes into `ALLOWED_NEW`, `TicketFile` and
  `.ai/tickets/schema.json` in one commit;
- never `status`, `order`, `shipped_at` or `completed_at` by hand: those belong to `reorder`,
  `set-status`, `mark-ready`, `ship` and `stamp-sha`, which check what they write;
- never the files `ticket sync` writes: `.ai/tickets/queue.json`, the next-work block of the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) roadmap and the ticket column
  of the Eden gap analysis.

`ticket check` validates the schema and the rules but not the key order; the test
`corpus_roundtrip_real_tree_byte_identical` does, and neither CI nor the wave gate runs it:

```bash
cargo test -p ticket-engine corpus_roundtrip_real_tree_byte_identical
```

Expected: one `test result: ok. 1 passed` line; the other test binaries of the crate report 0
passed and the test filtered out. A failure names the ticket file whose rendering differs.

## Verify

```bash
cargo xtask ticket check --strict
```

Expected: `check OK`, with the ticket shipped and stamped. `cargo xtask ticket show <ticket id>`
shows it `shipped`, and `cargo xtask ticket metrics` lists its receipt as
`<id>  agent=<program>  started=<time>  elapsed_sec=<n>  tokens.total=<n>  outcome=ran`, or
prints `(no run files under .ai/tickets/metrics/)` when the work ran outside `slice-run`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `No ready tickets. Steps:` and exit 1 | no `ready` ticket with a spec and the `claude-code` executor in `queue.json` | steps 3 to 5; the hint's "Composer" and `T-0xx` lines are stale wording, the steps are these |
| `ticket run --stream <S>` always finds no ticket | ticket files carry no `stream` key, so no ticket matches | run without `--stream`, or run one ticket with `platform slice-run` |
| `[<id>] SKIP — spec missing: <spec>` | the spec path in the ticket names no file | fix the `spec` field with `mark-ready <id> <spec path>` |
| `[<id>] refusing slice-run: executor is <x> (not claude-code)` | the ticket or its active slice is for a person, [Workbench](/documentation_v2/glossary/n_to_z.md#workbench), CI or a documentation pass | do that work by hand; ship as in step 9 |
| `agent CLI stdout is not JSON — cannot extract usage, run FAILED` | the agent command prints text, not its JSON answer | set `TBD_SLICE_RUN_AGENT_CMD` to a command with JSON output |
| `[<id>] run FAILED — no metrics file written` | the answer had no usage object | the run did not count; re-run it, or finish by hand and land with `--bookkeeping` in a wave |
| `Plan file not found: <path> — nothing goes ready without its own plan document; …` | step 4's plan is missing | copy the plan template to the path it names |
| `refusing mark-ready <id>: ready requires order and the ticket has none …` | the ticket is still an `idea` | step 3 |
| `refusing mark-ready <id>: ready-tier body fields empty: <fields> …` | the body is not filled | step 2 |
| `Blocked by <id> (status=<status>)` | a `depends_on` ticket is not shipped or cancelled | ship or cancel it first, or drop the dependency |
| `xtask: refusing <verb> <id>: ticket check failed (<n> error(s))` | a ticket file is invalid, or a shipped ticket still lacks `shipped_at` | `cargo xtask ticket check` names each error; stamp a pending ship first (step 11) |
| `refusing ship <id>: created_at is absent …` | an old ticket was never stamped at birth | add `created_at` by hand: the UTC author date of the file's first commit |
| the agent committed on `main` in the main checkout | no slice worktree existed, so it ran at the root | create the worktree before the run (step 7) when the work needs a branch |

## Related

- [Ticket command group](/tools_v2/xtask/src/commands/ticket/README.md) — every `ticket`
  subcommand, its flags and exit codes.
- [Platform command group](/tools_v2/xtask/src/commands/platform/README.md) — `slice-run`,
  `slice-worktree` and the wave driver.
- [Ticket engine](/tools_v2/ticket-engine/README.md) — the crate behind the ticket files, the
  queue and the run receipts.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — the ticket fields, spec
  and plan names, and ids in commit subjects.
- [Commit checklist](/documentation_v2/standards/commit_checklist.md) — what the landing commit
  carries.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the same lifecycle for many
  tickets in parallel, with gates, landing and a verifier.
- [Ticket registry](/.ai/tickets/README.md) — the ticket folder and its templates.
