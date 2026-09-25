# Ticket command group

The `cargo xtask ticket` group and the top-level `cargo xtask registry-get`: every read and write
of the [ticket](/documentation_v2/glossary.md#ticket) registry, the TOML files under
`.ai/tickets/`. Agents, the command center, the ticketboard and people run them; the storage,
validation, sync and shipping logic lives in the `ticket-engine` crate, and this folder adds the
side effects that need xtask: running an agent and deleting worktrees and branches.

## Contents

```text
tools_v2/xtask/src/commands/ticket/
├── cli.rs        the `TicketCmd` clap enum: twenty-seven subcommands and their flags
├── dispatch.rs   loads the registry and calls the ticket-engine command for each subcommand
├── execution.rs  `clean`, `done` and `run`: worktree and branch removal, and slice runs through the agent
├── mod.rs        the module tree; re-exports the ticket-engine commands and `load_registry`
└── tests/        unit tests for the cleanup of an unregistered worktree and cleanup before a refused ship
```

## How it works

`tools_v2/xtask/src/cli/mod.rs` mounts `TicketCmd` as the `ticket` group. `dispatch::run` finds
the checkout root, loads the registry with `ticket_engine::registry::load_registry` (every verb
but `stamp-sha`, `gap-round-trip`, `metrics` and `scope-histogram`, which read the files
themselves), calls one `cmd_*` function of `ticket_engine::cli`, `sync`, `validation` or `metrics`,
and returns 0. `registry-get` is dispatched in `tools_v2/xtask/src/cli/dispatch.rs` and uses the
same `load_registry`.

Every verb that writes a ticket first runs the full `ticket check` and refuses while it is red,
then writes the ticket files, reloads the registry and refreshes the derived files:

| Verb | Refreshes |
|---|---|
| `add`, `add-child`, `remove`, `reorder`, `advance-slice`, `mark-ready` | `ticket sync` |
| `set-status` | `.ai/tickets/queue.json` only, then repacks `.ai/tickets/wave.lock` |
| `ship` | `ticket sync`, then repacks `.ai/tickets/wave.lock` unless `--no-repack` |
| `stamp-sha` | nothing; it runs without the check, since it is the step that turns the tree green again after a ship |

`clean`, `done` and `run` are the side effects kept here (`execution.rs`). `clean` removes the
ticket's worktree, `TBD-<id>` under the configured `worktree_base` (`git worktree remove --force`,
then a plain delete), and deletes its branch, `ticket/<id>` unless the ticket names another; it
never creates a branch. `done` runs `clean` and then `ship`. `run` hands each ready ticket to
`slice_execution::run_slice` in `tools_v2/xtask/src/commands/platform/`.

## Commands

Run each as `cargo xtask ticket <subcommand>` from the repository root. Exit codes are shared: 0
done; 1 an unknown ticket (`Unknown ticket: <id>`), a red `ticket check`, a refused write or any
other error (`xtask: <cause>`); 2 a clap usage error.

### Reading the registry

- Synopsis: `ticket show <ID>`; `ticket get <ID> [FIELD]`; `ticket brief <ID>`;
  `ticket prompt <ID> [--slice <S>] [--header]`; `ticket next`; `ticket list`;
  `ticket plan-batch`; `ticket ready-ids [--limit <N>] [--stream <S>]`; `ticket config <KEY>`;
  `ticket sparse-paths <ID>`; `ticket scope-histogram`; `ticket metrics [--by agent]`;
  `registry-get <FIELD>`.
- Does: `show` prints a ticket's summary card and `get` its JSON or one field; `brief` prints
  what an executor reads first and `prompt` the prompt block of its (slice) spec; `next` shows
  the active slice and the next five open tickets, `plan-batch` the next ten, and `list` the
  queue in `.ai/tickets/queue.json`; `ready-ids` lists ready tickets with a spec and the
  `claude-code` executor; `config` reads `batch_size` (10), `concurrency` (3), `worktree_base` or
  `git_base` (`main`); `sparse-paths` lists the folders a sparse checkout of the ticket needs;
  `scope-histogram` counts the typed corpus by domain, layer, component and surface; `metrics`
  sums the run receipts in the `metrics` folder of `.ai/tickets/`; `registry-get` prints a top-level registry
  field such as `next_id`.
- Example: `cargo xtask ticket show <ticket id>`

### Checking and syncing

- Synopsis: `ticket check [--strict]`; `ticket sync`; `ticket gap-round-trip`.
- Does: `check` validates every ticket against `.ai/tickets/schema.json` and the structural rules,
  checks `.ai/tickets/wave.lock` against the tickets, prints the debt counters (with `--strict`
  also the measured and estimated token counters), and prints `check OK`. `sync` writes
  `.ai/tickets/queue.json` and, when the files exist, the next-work block of the product roadmap
  and the ticket column of the gap analysis. `gap-round-trip` proves that the gap-analysis table
  reads and writes back byte for byte.
- Example: `cargo xtask ticket check --strict`

### Creating and changing tickets

- Synopsis: `ticket add <TITLE> [--summary <S>]`; `ticket add-child <PARENT> <TITLE> [--summary <S>] [--promote]`;
  `ticket remove <ID> [--force]`; `ticket reorder <ID> <AFTER>`; `ticket set-status <ID> <STATUS>`;
  `ticket mark-ready <ID> [SPEC] [PLAN]`; `ticket advance-slice <ID>`.
- Does: `add` mints the next parent id as a work ticket with status `idea`; it also accepts
  `--program`, `--surfaces` and `--impact` but ignores them. `add-child` mints the next dotted
  child and, with `--promote`, turns a work parent into a program. `remove` deletes one ticket, and
  a program only with `--force`, which deletes its descendants. `reorder` places a ticket after
  another and moves an `idea` to `queued`. `set-status` accepts `idea`, `queued`, `ready`,
  `running`, `review`, `shipped`, `deferred` or `cancelled`. `mark-ready` records the spec and
  requires the plan, by default
  `documentation_v2/tickets/plans/<id, lowercased, dots as underscores>_plan.md`, to exist.
  `advance-slice` moves a program's active slice to its next child.
- Example: `cargo xtask ticket set-status <ticket id> queued`

### Shipping

- Synopsis: `ticket ship <ID> [--no-repack]`; `ticket stamp-sha <ID> <SHA>`; `ticket done <ID>`;
  `ticket clean <ID>`.
- Does: `ship` marks a ticket shipped, stamps `completed_at` and clears it as an active slice;
  `--no-repack` leaves the wave lock stale so a whole wave can ship before one
  `cargo xtask wave repack`. After the landing commit, `stamp-sha` writes its sha to `shipped_at`
  (the same sha again changes nothing, another is refused) and writes a token estimate when the
  ticket has neither a run receipt nor an estimate. `done` is `clean` then `ship`; `clean` alone
  removes the worktree and branch.
- Example: `cargo xtask ticket stamp-sha <ticket id> <short sha>`

### Running

- Synopsis: `ticket run [--dry-run] [--stream <S>]`
- Does: takes up to `batch_size` ready tickets with a spec and the `claude-code` executor (of one
  stream with `--stream`) and runs each in turn through `platform slice-run`, which writes a run
  receipt; `--dry-run` prints each ticket's branch and spec and runs nothing.
- Exit codes: as above, and 1 when no ticket is ready.
- Example: `cargo xtask ticket run --dry-run`

## Boundaries

- Depends on: `ticket_engine` (`cli`, `registry`, `sync`, `validation`, `metrics`);
  `crate::core::repository_root`; `crate::commands::platform::slice_execution` for `run`; `git`
  for `clean`.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, and `tools_v2/xtask/src/main.rs`, which imports
    `load_registry`;
  - `tools_v2/xtask/src/commands/platform/dispatch.rs`, which loads the registry for
    `platform slice-run`;
  - the platform preflight and wave gate and the mod wave gate, which run `ticket check`;
  - the `language-gates` job of `.github/workflows/ci.yml` (`ticket check --strict`);
  - the ticketboard (`apps/ticketboard/`), whose every change is a `cargo xtask ticket` command;
    agents, the command center and people.
- Rules:
  - Ticket storage, validation, sync and wave packing stay in `ticket-engine`; `mod.rs` must
    delegate to it (`ticket_implementations_have_one_owner` in
    `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`).
  - `clean` removes a worktree even when git does not know it
    (`cleanup_removes_an_unregistered_worktree_directory` in `tests/execution_tests.rs`), and
    `done` cleans before a ship that is refused (`done_cleans_before_a_shipping_refusal`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, their schema and the derived files.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — how ticket ids are
  formed and cited.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the ship, stamp and repack
  order during a wave.
