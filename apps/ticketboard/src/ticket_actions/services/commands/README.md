# Ticket command rules

The egui-free rules behind every change the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)
makes to a [ticket](/documentation_v2/glossary/n_to_z.md#ticket): one builder per `cargo xtask ticket`
verb, the file-change guard, the single-flight queue, and which transitions each status offers.

## Contents

```text
apps/ticketboard/src/ticket_actions/services/commands/
├── file_change_guard.rs  `FileChangeGuard`, the FNV-1a fingerprint of a ticket file, and `cas_ok`
├── mod.rs                the module tree; re-exports every item of the four files
├── queue.rs              `TicketCommandQueue`, one command at a time with a FIFO tail, and `Finish`
├── requests.rs           `TicketCommand`, `TICKET_PREFIX` and one builder per ticket verb
├── tests/                unit tests for the builders, queue, guard, transition matrices and hints
└── transitions.rs        actions per status, the recovery hint, the success tail and the remove gates
```

## How it works

A `TicketCommand` carries the argument list handed to `cargo`, the command line the viewer
confirms, and an optional `FileChangeGuard`. The arguments are `TICKET_PREFIX`
(`run --package xtask -- ticket`, the expansion of the `cargo xtask` alias) followed by the verb
tail, so the command runs without alias resolution. The display line starts `cargo xtask ticket`
and single-quotes any argument outside a plain file-name alphabet; no shell ever parses it.

The builders are `ship`, `set_status` (any of the eight statuses), `mark_ready` (the spec
optional), `reorder`, `add` and `add_child` (`--summary` only when the summary is not blank,
`--promote` when asked), `remove` (`--force` when asked) and `advance_slice`, one per verb.

The guard is taken when a menu renders or a button is clicked: the path of the ticket file and an
FNV-1a hash of its bytes, content-based because timestamps and lengths miss edits. `cas_ok`
re-hashes at dispatch and passes only on an equal hash (two absent files are equal, so the verb's
own "unknown ticket" refusal shows); a command without a guard (`add`) always passes.

`TicketCommandQueue::submit` hands back the command to start when idle and parks it otherwise.
`finish(success)` pops the next command on success; on failure it drops the whole pending tail
and reports how many it dropped, and nothing is retried. A queued command that the guard refuses
is finished as a success, since no verb failed.

`transitions.rs` holds the status rules. Both matrices match every `StatusName` variant without a
wildcard, so a ninth status breaks the build.

| Status | Offered transitions | Verb |
|---|---|---|
| `idea` | "Queue after…" | `reorder`, which the verb turns into `queued` |
| `queued` | "Mark ready…" | `mark-ready` |
| `ready`, `review` | "Ship…", "Demote to queued", "Defer", "Cancel" | `ship`, `set-status` |
| `running` | none | the runner owns the ticket |
| `shipped`, `deferred`, `cancelled` | "Reopen to queued" | `set-status <id> queued` |

`advanced_actions` offers the raw `set-status` to every ticket, `advance-slice` to programs,
`remove` to all, and a cancel to a `running` ticket only. `recovery_hint_applies` is true when a
command was killed by a signal or its output names `cargo xtask wave repack`; the hint is the text
`RECOVERY_HINT`, never a button. `success_tail` picks the last output line that is neither blank
nor cargo's own build noise. `remove_gate_ok` wants the exact id typed, surrounding spaces
forgiven; `descendants` lists the corpus ids that extend an id with a dot, in numeric order.

## Boundaries

- Depends on: `crate::ticket_registry::models::projection` (`id_sort_key`); `ticket_engine`
  (`StatusName`); `std::fs` to read ticket files for the fingerprint.
- Used by: the rest of `crate::ticket_actions`; `crate::application`, whose
  `command_execution.rs` checks `cas_ok`, drives the queue and applies the hint and success tail;
  `apps/ticketboard/src/application/tests/rendering.rs`.
- Rules:
  - every argument list is `TICKET_PREFIX` plus one verb tail, with the flags only when set
    (`ship_builder_argv_and_display`, `add_child_flag_combos`, `remove_force_combo` in
    `tests/commands.rs`);
  - the queue runs one command, keeps each parked command's own guard, and drops the tail on
    failure without retry (`queue_is_single_flight_fifo`,
    `queue_failure_drops_the_pending_tail_and_never_retries`,
    `queued_requests_keep_their_captured_fingerprints`);
  - `running` is never a target of an offered transition
    (`running_is_never_a_dispatch_target_in_the_normal_ui`, `offered_transitions_matrix_pinned`);
  - the recovery hint stays text: the viewer never runs `cargo xtask wave repack`
    (`recovery_hint_triggers_on_the_wave_stale_signature_only`).

## Related documentation

- [Ticket commands](/tools_v2/xtask/src/commands/ticket/README.md) — the `cargo xtask ticket`
  verbs these builders call.
