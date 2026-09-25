# Ticket action services

The egui-free half of the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s ticket
actions: the command rules, and the constructors that open each mutation dialog with the data and
file-change guard it needs.

## Contents

```text
apps/ticketboard/src/ticket_actions/services/
├── commands/           verb builders, file-change guard, single-flight queue and transition rules
├── dialog_builders.rs  constructors for every `Dialog`, the ready-spec prefill, anchor candidates
└── mod.rs              the module tree
```

## How it works

`commands/` decides what may run and builds it. `dialog_builders.rs` turns a click into a `Dialog`
from a `TicketActionContext`, the corpus and the id index, and never from application state.

- `transition_dialog` opens the anchor picker for "Queue after…", the ready form for "Mark
  ready…", and a confirmation holding the one command of every other transition, titled with the
  label and the id.
- `ready_spec_prefill` fills the ready form with the ticket's own spec, else its parent's spec,
  else nothing.
- `anchor_dialog` opens the anchor picker for a card dropped on the `queued` column, taking the
  guard at drop time; `add_dialog`, `add_child_dialog` and `remove_dialog` open the other forms,
  and `add_child_dialog` records whether the parent is a work ticket.
- `anchor_candidates` lists every other ticket that carries an order, sorted by order then numeric
  id, filtered on the lowercase id and title.

Every constructor for an existing ticket stores the guard its caller took; `add_dialog` has no
target file and takes none.

## Public surface

- `commands`: `TicketCommand`, `FileChangeGuard`, `TicketCommandQueue`, the verb builders,
  `cas_ok`, `recovery_hint_applies` and `success_tail`, which `crate::application` uses to run
  commands.
- `dialog_builders::anchor_dialog`: the drop on the `queued` column, opened by
  `crate::application`'s action dispatch; `dialog_builders::add_dialog`: the toolbar's
  "New ticket…".

## Boundaries

- Depends on: `crate::ticket_actions::models` (`Dialog`, `TicketActionContext`);
  `crate::ticket_registry::models` (`Corpus` and the `projection` view, `id_sort_key`);
  `ticket_engine` (`StatusName`, `Ticket`).
- Used by: `crate::ticket_actions::ui` (menus and dialogs); `crate::application`
  (`command_execution.rs`, `action_dispatch.rs`, `ticket_command_views.rs`, `mod.rs`) and
  `apps/ticketboard/src/application/tests/rendering.rs`.
- Rules: nothing here names egui (the test
  `dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`); nothing here writes a ticket file, since
  every change is a `cargo xtask ticket` command.
