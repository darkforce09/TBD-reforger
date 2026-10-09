# Ticket actions

The [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) feature that changes
[tickets](/documentation/glossary/n_to_z.md#ticket): it builds each `cargo xtask ticket` command,
guards it against a ticket file changed on disk, queues commands one at a time, and models the
dialogs, the running command and the toasts that `tools/tickets/ticketboard_desktop/src/ticket_actions/ui/` paints.
It never writes a ticket file itself.

## Contents

```text
tools/tickets/ticketboard_model/src/ticket_actions/
├── events.rs  `TicketActionEvent`: open a dialog, dispatch a command, toggle the command drawer
├── mod.rs     the module tree
├── models/    the running command and its output, the open dialog, the mutation context, toasts
└── services/  verb builders, file-change guard, queue, transition rules and dialog constructors
```

## How it works

A change runs in four steps, and the application drives the ones that touch processes:

```text
card menu / action strip ──▶ Dialog (guard taken) ──"Run"──▶ Dispatch(TicketCommand)
                                                                   │ application
cas_ok? ──no──▶ toast "file changed on disk — reloading", reload   │
   │ yes                                                           ▼
TicketCommandQueue ──idle──▶ cargo run --package xtask -- ticket <verb> …  (in the repository root)
   │ busy: parked FIFO                                             │ exit
   ◀──────────────── finish: next on success, drop the tail on failure
```

1. The desktop application's card menu and action strip
   (`tools/tickets/ticketboard_desktop/src/ticket_actions/ui/menus.rs`) offer the transitions the ticket's status
   allows (`services/commands`), and each opens a dialog from `services/dialog_builders.rs` carrying the `FileChangeGuard` of the ticket
   file.
2. The dialog shows the exact command line; "Run" emits `TicketActionEvent::Dispatch`.
3. The application re-hashes the file with `cas_ok` and refuses and reloads on a mismatch;
   otherwise the queue starts the command or parks it behind the running one. While a command
   runs, every control that could dispatch is disabled.
4. On exit the application always reloads the corpus and `git status`. Success closes the drawer
   and toasts the last output line that is not cargo's; failure keeps the full output open, drops
   the queued commands without retry, requests one strict check, and shows
   `cargo xtask wave repack` as text when the command was killed or its output names it.

## Public surface

- `events::TicketActionEvent`: what the menus, dialogs and drawer emit; the application converts
  it into its own action, and the browser forwards it from the callbacks it is lent.
- `models`: `CommandExecutionState`, `Dialog`, `MutationContext`, `TicketActionContext` and
  `Toast`, which the application owns and lends back.
- `services::commands`: `TicketCommand`, the verb builders, `cas_ok`, the queue,
  `recovery_hint_applies` and `success_tail`, for the application's command execution;
  `services::dialog_builders::add_dialog` and `anchor_dialog`.

The menus, dialogs and feedback renderers that paint these models live in the desktop application,
under `tools/tickets/ticketboard_desktop/src/ticket_actions/ui/`.

## Boundaries

- Depends on: `crate::core::process::ProcessHandle`; `crate::ticket_registry::models` (`Corpus`,
  the `projection` view and sort keys); `ticket_model` (`StatusName`, `Ticket`); at run time
  `cargo xtask ticket`, which the desktop application spawns.
- Used by: `crate::application_state::events`, which converts `TicketActionEvent` into an
  `Action`; `crate::ticket_browser`, whose `BrowserEvent::TicketAction` carries it; the desktop
  application: `tools/tickets/ticketboard_desktop/src/ticket_actions/ui/` and `tools/tickets/ticketboard_desktop/src/application/`
  (`mod.rs`, `command_execution.rs`, `action_dispatch.rs`, `feature_views.rs`,
  `ticket_command_views.rs`).
- Rules:
  - every change is a `cargo xtask ticket` command; the feature writes no ticket file and never
    runs `cargo xtask wave repack`;
  - nothing here names egui, and dialogs see only the corpus and the id index, never application
    state;
  - a failed command drops the queued ones and nothing retries.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, statuses and commands these
  actions run.
- [Ticket commands](/tools/xtask/src/commands/ticket/README.md) — the `cargo xtask ticket`
  verbs.
