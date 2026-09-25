# Ticket action rendering

The egui controls of the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s ticket actions:
the card context menu and the detail panel's action strip, the mutation dialogs, and the feedback
of a running command (footer chip, drawer and toasts).

## Contents

```text
apps/ticketboard/src/ticket_actions/ui/
├── dialogs/     the modal mutation forms, each showing its exact command line before "Run"
├── feedback.rs  toasts, the footer command chip and the command drawer with the full output
├── menus.rs     the card context menu and the detail panel's action strip with its Advanced section
└── mod.rs       the module tree and the dialog, list, drawer and anchor-row sizes
```

## How it works

Nothing here runs a command. Each control pushes a `TicketActionEvent` (open a dialog, dispatch a
command, toggle the drawer), which the application turns into an action after the frame. Every
control that could dispatch is disabled while `MutationContext::busy` is true.

- `card_menu_ui` is the context menu of a board card: the id and status, the transitions its
  status offers, and "Add child…". A `running` ticket offers no transition and points to the
  detail panel. The file-change guard is taken each frame the menu is open, so a click carries the
  latest fingerprint.
- `action_strip_ui` is the row under a ticket's details: the same transitions and "Add child…",
  then a collapsed "Advanced" section with a raw `set-status` picker over all eight statuses and
  "Dispatch…", "Advance slice…" for programs, "Remove…", and "Cancel (running)…" for a `running`
  ticket only. Here the guard is taken at the click.
- `feedback.rs` paints the toasts in the top-right corner until they expire; the footer chip,
  which shows the running command with a spinner and the pending count, or the last outcome
  ("exit N", "killed" or "verb did not run"), and toggles the drawer; and the drawer, which shows
  the outcome, the note on dropped requests, the recovery hint as text when it applies, and the
  full output in a virtualized list that follows the newest line.
- `dialogs/` paints the open dialog.

## Public surface

- `menus::card_menu_ui` and `menus::action_strip_ui`: passed by `crate::application` to the
  browser as the card-menu and action-strip callbacks.
- `dialogs::dialog_ui`, `feedback::toasts_ui`, `feedback::verb_chip_ui` and `feedback::drawer_ui`:
  painted by `crate::application::ticket_command_views`.

## Boundaries

- Depends on: `crate::ticket_actions::models`, `crate::ticket_actions::services` and
  `crate::ticket_actions::events`; `crate::ticket_registry::models::projection` (`STATUS_ORDER`,
  the ticket view); `crate::core::ui` (the verdict colours); `ticket_engine` (`StatusName`,
  `Ticket`); `eframe::egui`.
- Used by: `crate::application` (`feature_views.rs` and `ticket_command_views.rs`); the browser
  reaches the menus only through those callbacks.
- Rules: no other feature imports this module (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); the recovery command
  `cargo xtask wave repack` appears as text only, never as a button.
