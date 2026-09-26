# Mutation dialogs

The modal forms through which the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) confirms
every change to a [ticket](/documentation_v2/glossary/n_to_z.md#ticket): each shows the exact
`cargo xtask ticket` command line it will run and emits it only when the viewer presses "Run".

## Contents

```text
apps/ticketboard/src/ticket_actions/ui/dialogs/
├── anchor_picker.rs    "Queue <id> after…": a filterable list of ordered tickets for `reorder`
├── confirmation.rs     a one-command confirmation with an optional note
├── mark_ready.rs       "Mark <id> ready": the spec path, checked on disk as typed
├── mod.rs              `dialog_ui`, the modal per variant, the command line, "Run" and "Cancel"
├── removal.rs          "Remove <id>": `--force` with its descendants listed, type the id to confirm
└── ticket_creation.rs  "New ticket" and "Add child under <parent>", `--promote` for a work parent
```

## How it works

`dialog_ui` paints the open `Dialog` in an egui `Modal` at least 460 points wide and hands each
variant to its body. It returns true when the dialog should close: on "Cancel", Escape, a click on
the backdrop, or a dispatch. Every body ends with `command_line_ui`, which shows "This will run:"
and the command line, and `run_cancel_ui`, whose "Run" pushes `TicketActionEvent::Dispatch`.
"Run" stays disabled while a command runs, with the note "a verb is already running — dispatch
disabled until it exits", and until the form is valid:

| Dialog | Run enabled when |
|---|---|
| confirmation | always |
| anchor picker | an anchor is selected |
| ready form | always; a blank or missing spec path shows the refusal the verb will give |
| new ticket, add child | the title is not blank |
| removal | the typed text equals the id |

The anchor picker lists `anchor_candidates` in a virtualized list and explains that the verb sets
the order to the anchor's plus one and turns `idea` into `queued`. The ready form re-checks the
spec path on disk only when its text changes, and shows the ticket's current `main_goal` and
`acceptance` read-only, since the verb takes only the id and spec and fills them itself when they
are empty. The removal form lists, under `--force`, every corpus id that extends the ticket's id
with a dot. The add-child form offers `--promote` only when the parent is a work ticket, which the
verb then turns into a program.

Refusals are not predicted beyond these hints: each form notes that the verb's refusal streams
verbatim into the command drawer.

## Boundaries

- Depends on: `crate::ticket_actions::models` (`Dialog`, `MutationContext`,
  `TicketActionContext`) and `crate::ticket_actions::services` (the verb builders,
  `remove_gate_ok`, `descendants`, `anchor_candidates`); `crate::ticket_registry::models::projection`
  (the ticket view, `truncate_chars`); `crate::core::ui` (`VERDICT_OK`, `VERDICT_COLLIDE`); the
  sizes in `crate::ticket_actions::ui`; `eframe::egui`.
- Used by: `crate::application::ticket_command_views`, which calls `dialog_ui` each frame a
  dialog is open.
- Rules: a dialog dispatches only the command whose line it shows, carrying the guard its dialog
  was opened with; `apps/ticketboard/src/application/tests/rendering.rs` paints every dialog
  headlessly without running a command.
