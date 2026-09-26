# Ticket registry

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s shared
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) data: it finds the repository root, loads every
parent and child ticket from `.ai/tickets/` or refuses with the file at fault, and gives every
other feature the same ticket fields, status columns, classes, scope breadcrumbs and colours.

## Contents

```text
apps/ticketboard/src/ticket_registry/
├── mod.rs     the module tree
├── models/    the corpus and its refusal, and the shared ticket projections, scope and palettes
└── services/  repository-root discovery and the all-or-nothing corpus load
```

## How it works

At start, `main.rs` takes the positional argument through `services::discovery`, and the
application resolves the root with it, falling back to the walk up from the working directory. The
application's loading thread then calls `services::corpus_loading::load_corpus`, which parses
every `T-*.toml` file with `ticket_engine::parse_ticket_toml` and returns a `Corpus` or a
`LoadError`; a refusal replaces the board with a screen naming the file and the error, until a fix
on disk and a reload load it again. The wave lock, receipts, estimates and scope vocabulary load
beside the corpus in `crate::application`, not here, so their failures stay local to their own
displays.

Every feature reads tickets through `models::projection`, the one place that knows how the two
ticket kinds, program and work, hold each field, and through its colour and breadcrumb helpers.
The raw lowercase status and class names are the labels everywhere.

## Public surface

- `services::discovery`: `positional_arg` for `main.rs`; `resolve_repo_root`, `has_tickets_dir`
  and `walk_up_for_tickets` for `crate::application` and the live tests.
- `services::corpus_loading::load_corpus` and `is_child_id`, for the application's loading thread
  and the test fixtures.
- `models::corpus`: `Corpus`, `LoadedTicket`, `Counts`, `LoadError`, `LoadResult`, read by every
  feature.
- `models::projection`: `TicketView`, `view`, `STATUS_ORDER`, `column_of`, `id_sort_key`, the
  field helpers, `Class` and `breadcrumb`; `models::palette`: `status_rgb` and `scope_level_rgb`.

## Boundaries

- Depends on: `ticket_engine` (`Ticket`, `Status`, `StatusName`, `ScopeV2`, `parse_ticket_toml`,
  `repository::TICKETS_DIR`); `std::fs`. It depends on no other ticketboard module.
- Used by: `apps/ticketboard/src/main.rs`; `crate::application`; `crate::ticket_browser`,
  `crate::ticket_actions`, `crate::wave_plan` and `crate::execution_metrics`;
  `apps/ticketboard/src/tests/support/mod.rs`.
- Rules:
  - the registry imports no consuming feature, only `core` besides itself, and `models/` and
    `services/` name no egui type
    (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`);
  - a load is complete or refused, never partial, and never writes a ticket file
    (`apps/ticketboard/src/ticket_registry/services/tests/corpus_loading.rs`);
  - the load checks each file's shape only; scope-vocabulary legality and the id-to-file-name match
    are left to `cargo xtask ticket check --strict`, whose verdict the status banner shows.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, fields and statuses the corpus
  holds.
