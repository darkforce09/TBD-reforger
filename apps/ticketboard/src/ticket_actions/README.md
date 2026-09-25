# Ticket actions

The [ticketboard](/documentation_v2/glossary.md#ticketboard) feature that changes
[tickets](/documentation_v2/glossary.md#ticket): it builds each `cargo xtask ticket` command,
guards it against a ticket file changed on disk, queues commands one at a time, and renders the
menus, dialogs and command feedback. It never writes a ticket file itself.

## Contents

```text
apps/ticketboard/src/ticket_actions/
├── events.rs  `TicketActionEvent`: open a dialog, dispatch a command, toggle the command drawer
├── mod.rs     the module tree
├── models/    the running command and its output, the open dialog, the mutation context, toasts
├── services/  verb builders, file-change guard, queue, transition rules and dialog constructors
└── ui/        card menu, action strip, mutation dialogs, command chip, drawer and toasts
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

1. `ui/menus.rs` offers the transitions the ticket's status allows (`services/commands`), and each
   opens a dialog from `services/dialog_builders.rs` carrying the `FileChangeGuard` of the ticket
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
- `ui::menus::card_menu_ui` and `ui::menus::action_strip_ui`, handed to the browser as callbacks;
  `ui::dialogs::dialog_ui` and the `ui::feedback` renderers.

## Boundaries

- Depends on: `crate::core` (`process::ProcessHandle`, the `ui` colours);
  `crate::ticket_registry::models` (`Corpus`, the `projection` view and sort keys); `ticket_engine`
  (`StatusName`, `Ticket`); `eframe::egui` in `ui/` only; at run time `cargo xtask ticket`, which
  the application spawns.
- Used by: `crate::application` (`mod.rs`, `events.rs`, `command_execution.rs`,
  `action_dispatch.rs`, `feature_views.rs`, `ticket_command_views.rs`) and its
  `tests/rendering.rs`; `crate::ticket_browser`, whose `BrowserEvent::TicketAction` and callback
  types carry `TicketActionEvent`.
- Rules:
  - every change is a `cargo xtask ticket` command; the feature writes no ticket file and never
    runs `cargo xtask wave repack`;
  - `models/` and `services/` never name egui, and dialogs see only the corpus and the id index,
    never application state (the test
    `dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`);
  - a failed command drops the queued ones and nothing retries
    (`queue_failure_drops_the_pending_tail_and_never_retries` in
    `services/commands/tests/commands.rs`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, statuses and commands these
  actions run.
- [Ticket commands](/tools_v2/xtask/src/commands/ticket/README.md) — the `cargo xtask ticket`
  verbs.
