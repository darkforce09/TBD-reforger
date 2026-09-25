# Ticket action state

The data the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s ticket actions keep
between frames: the running command and its output, the one open mutation dialog, the read-only
context every control needs, and the toasts.

## Contents

```text
apps/ticketboard/src/ticket_actions/models/
├── command_execution.rs  `CommandExecutionState` (queue, process, log, drawer) and `CommandOutcome`
├── dialog.rs             `Dialog`, the one open mutation dialog, one variant per form
├── mod.rs                the module tree; re-exports every item of the four files
├── mutation_context.rs   `MutationContext` (root, busy flag) and `TicketActionContext` (corpus, ids)
└── notification.rs       `Toast`, a message that expires six seconds after it is made
```

## How it works

`CommandExecutionState` is owned by the application: the `TicketCommandQueue`, the
`ProcessHandle` of the running command, its whole merged output (never truncated, so a refusal
always shows in full), whether the drawer is open, the note on requests dropped after a failure,
and the last `CommandOutcome`. The outcome holds the command line, the exit code (`None` when a
signal ended the process), the finish time as `HH:MM:SS UTC`, a spawn error when the command never
ran, and whether the recovery hint shows.

`Dialog` has six variants: `Confirm` (a title, an optional note and the command), `AnchorPick`,
`MarkReady` (with a cached existence check of the typed spec path), `AddTicket`, `AddChild` and
`Remove`. Every variant that targets an existing ticket carries the `FileChangeGuard` taken when
its control was used; `AddTicket` has none.

`MutationContext` is copied into every control: the repository root and `busy`, true while a
command runs, which disables every control that could dispatch. `TicketActionContext` borrows only
the corpus and the id-to-index map, so a dialog never reaches application state.

## Boundaries

- Depends on: `crate::core::process::ProcessHandle`; `crate::ticket_actions::services::commands`
  (`FileChangeGuard`, `TicketCommand`, `TicketCommandQueue`);
  `crate::ticket_registry::models::corpus::Corpus`.
- Used by: `crate::ticket_actions::services::dialog_builders` and `crate::ticket_actions::ui`;
  `crate::application` (`mod.rs`, `command_execution.rs`, `ticket_command_views.rs`), which owns
  the state; `apps/ticketboard/src/application/tests/rendering.rs`.
- Rules: no egui type appears here (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); at most one dialog is open at a time, as
  the application's `Option<Dialog>` field holds it.
