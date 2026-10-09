# Ticket command services

The body of every `cargo xtask ticket` subcommand: queries and briefs that read the registry
projection, mutations that change [ticket](/documentation/glossary/n_to_z.md#ticket) files through the
typed operations, shipping and the landing-commit stamp, batch selection and configuration. Each
command is a `cmd_*` function that takes the checkout root; xtask parses the arguments and adds the
side effects this crate leaves out: running an agent and deleting worktrees and branches.

## Contents

```text
tools/tickets/ticket_registry/src/verbs/
├── batch.rs             `cmd_get`, `cmd_config`, `cmd_run` with its executor callback, and `cleanup_targets`
├── brief.rs             `cmd_brief`: what an executor reads first, from the ticket's own fields
├── mod.rs               the module tree; re-exports every command
├── mutation_support.rs  corpus load, registry reload, wave lock refresh, verbatim refusals
├── mutations.rs         `cmd_add`, `cmd_add_child`, `cmd_remove`, `cmd_reorder` and `cmd_advance_slice`
├── prompt.rs            `extract_prompt`: the prompt block of a specification document
├── queries.rs           `show`, `next`, `list`, `prompt`, `scope-histogram` and the other reads
├── readiness.rs         `cmd_mark_ready`
├── shipping.rs          `cmd_ship`, `cmd_ship_opt` and `cmd_stamp_sha`, which writes a token estimate
└── status.rs            `cmd_set_status` and `cmd_ready_ids`
```

## How it works

Reading commands take the parents-only registry value from `crate::registry` and print. A
mutation runs in these steps:

```text
Corpus::load (every ticket file, children included) and require_check_ok
  (a red ticket check refuses, nothing written)
  → one crate::ops operation, then Corpus::write_back / delete_files
  → reload the registry from disk
  → cmd_sync (queue, roadmap block, gap column); set-status and ship also repack the wave lock
```

`set-status` regenerates only `queue.json` before its repack. `ship --no-repack`
(`cmd_ship_opt` with `refresh` off) uses `require_check_ok_deferring_repack` and skips the repack,
so a whole wave ships before one `cargo xtask wave repack`. `stamp-sha` runs no preflight and no
sync: it writes `shipped_at` through `ops::stamp_sha`, and, when the ticket has neither a run
receipt nor an estimate, writes a token estimate from the ticket's subject commits plus the landing
commit (`ticket_metrics::estimates::plan_estimate_for_id`, over the subjects
`ticket_model::commit_subjects` mines) and adds `tokens` to `estimated`.

`cmd_add` accepts `program`, `surfaces` and `impact` arguments and ignores them; the class comes
from the title and summary words. Some refusals print their text bare on stderr and exit 1
(`refuse_verbatim`), the form external callers read.

`cmd_run` takes up to `batch_size` ready tickets with a spec and the `claude-code` executor, of
one stream when asked, and hands each to the caller's executor callback in queue order, stopping
at the first failure; `--dry-run` calls nothing. `cleanup_targets` resolves a ticket's worktree
(`TBD-<id>` under `worktree_base`) and branch (`ticket/<id>` unless the ticket names one) and
deletes neither. `cmd_config` reads `batch_size`, `concurrency`, `worktree_base` and `git_base`
from `queue.json`, with the defaults when a key is absent.

## Public surface

- Every `cmd_*` function: mounted by `tools/xtask/src/commands/ticket/mod.rs` and called from
  its `dispatch.rs`, one per subcommand.
- `cmd_run`, `cleanup_targets` and `cmd_ship`: wrapped by `tools/xtask/src/commands/ticket/execution.rs`,
  which supplies the executor (`platform slice-run`) and performs the removals for `clean` and
  `done`.
- `stamp_sha_with_inputs`: the testable core of `stamp-sha`, taking the mined subjects and
  line counts as arguments.
- `prompt::extract_prompt`: the prompt block of a spec, which `cmd_prompt` prints.

## Boundaries

- Depends on: `crate::ops` and `ticket_model::Corpus` for every write; `crate::registry` for the
  reads; `crate::validation` for the preflights; `crate::sync`; `ticket_wave_lock::repack_quiet`;
  `ticket_metrics` for receipts and estimates; `repository_layout`; `git` for the subject mining.
- Used by: `tools/xtask/src/commands/ticket/` (the whole `ticket` group); nothing else calls a
  command directly. The ticketboard runs these commands as `cargo xtask ticket` processes.
- Rules:
  - a red `ticket check` refuses every mutation before anything is written;
  - the sync after a write reads the files the write produced, never the value from before it;
  - the batch waiver passes only the stale-lock findings and never a missing lock;
  - this crate never deletes a worktree or branch and never starts an agent.

## Related documentation

- [Ticket command group](/tools/xtask/src/commands/ticket/README.md) — every subcommand, its
  flags and exit codes.
- [Ticket registry](/.ai/tickets/README.md) — the files these commands read and write.
