# `ticket_actions/`

## Responsibility

Builds ticket CLI commands, guards them against changed files, manages the single-flight queue, and renders mutation controls and execution feedback.

## Public surface

`services::commands` owns argument arrays, file guards, queue behavior, and transition policy. `services::dialog_builders` builds forms from `TicketActionContext`. `models` owns command and dialog state. UI emits `TicketActionEvent` for the application to execute.

## Dependency rules

Command logic and dialog data contain no egui types. Dialogs receive only ticket data and the identifier index, never application state. All registry changes go through existing xtask commands; this feature does not write ticket files. Command failure clears pending work without automatic retry or repacking.

## Files

- [events.rs](events.rs) — Events.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/command_execution.rs](models/command_execution.rs) — Command execution.
- [models/dialog.rs](models/dialog.rs) — Dialog.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/mutation_context.rs](models/mutation_context.rs) — Mutation context.
- [models/notification.rs](models/notification.rs) — Notification.
- [services/commands/file_change_guard.rs](services/commands/file_change_guard.rs) — File change guard.
- [services/commands/mod.rs](services/commands/mod.rs) — Module interface and composition.
- [services/commands/queue.rs](services/commands/queue.rs) — Queue.
- [services/commands/requests.rs](services/commands/requests.rs) — Requests.
- [services/commands/tests/commands.rs](services/commands/tests/commands.rs) — Tests for commands.
- [services/commands/transitions.rs](services/commands/transitions.rs) — Transitions.
- [services/dialog_builders.rs](services/dialog_builders.rs) — Dialog builders.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [ui/dialogs/anchor_picker.rs](ui/dialogs/anchor_picker.rs) — Anchor picker.
- [ui/dialogs/confirmation.rs](ui/dialogs/confirmation.rs) — Confirmation.
- [ui/dialogs/mark_ready.rs](ui/dialogs/mark_ready.rs) — Mark ready.
- [ui/dialogs/mod.rs](ui/dialogs/mod.rs) — Module interface and composition.
- [ui/dialogs/removal.rs](ui/dialogs/removal.rs) — Removal.
- [ui/dialogs/ticket_creation.rs](ui/dialogs/ticket_creation.rs) — Ticket creation.
- [ui/feedback.rs](ui/feedback.rs) — Feedback.
- [ui/menus.rs](ui/menus.rs) — Menus.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
