# Ticket registry loading

How the [ticketboard](/documentation_v2/glossary.md#ticketboard) finds the repository and reads
its [tickets](/documentation_v2/glossary.md#ticket): the repository-root discovery, and the
all-or-nothing load of every ticket file.

## Contents

```text
apps/ticketboard/src/ticket_registry/services/
├── corpus_loading.rs  `load_corpus` and `is_child_id`; re-exports the corpus types
├── discovery.rs       `positional_arg`, `resolve_repo_root`, `walk_up_for_tickets` and `has_tickets_dir`
├── mod.rs             the module tree
└── tests/             unit tests for discovery, the counts, fail-closed refusals and the live corpus
```

## How it works

`positional_arg` takes the first command-line argument that does not start with `-`.
`resolve_repo_root` returns that argument unchecked when given, so a named root without
`.ai/tickets/` is refused by the caller instead of replaced; otherwise it walks up from the working
directory to the first folder holding `.ai/tickets/`.

`load_corpus(repo_root)` lists the `T-*.toml` files in `.ai/tickets/`, sorted by name, and parses
each with `ticket_engine::parse_ticket_toml`. The first file that cannot be read or parsed, shape
errors such as an `idea` carrying `order` included, refuses the whole load as a `LoadError` naming
that file; a missing `.ai/tickets/` refuses with its path. A dotted id is a child and an undotted
one a parent. The load checks neither the scope vocabulary nor that a ticket's id matches its
file name, which `ticket_engine::Corpus::load` and `cargo xtask ticket check` do; the strict-check
banner reports those.

## Boundaries

- Depends on: `crate::ticket_registry::models::corpus`; `ticket_engine` (`parse_ticket_toml` and
  `repository::TICKETS_DIR`); `std::fs`.
- Used by: `apps/ticketboard/src/main.rs` (`positional_arg`);
  `apps/ticketboard/src/application/lifecycle.rs` and `background_events.rs` (`resolve_repo_root`,
  `has_tickets_dir`); `apps/ticketboard/src/application/background_loading.rs` (`load_corpus`);
  the ignored live tests of `crate::execution_metrics` and `crate::wave_plan`
  (`walk_up_for_tickets`); `apps/ticketboard/src/tests/support/mod.rs` and the ticket browser's
  tests (the corpus types and `is_child_id`).
- Rules: a load is all or nothing, naming the refusing file with its error verbatim
  (`fail_closed_names_the_bad_file`, `fail_closed_on_semantic_error_too` in
  `tests/corpus_loading.rs`); parents and children add up to the total
  (`counts_match_the_scratch_corpus`); the command-line root wins even when it is invalid
  (`arg_wins_even_when_invalid` in `tests/discovery.rs`); nothing here writes a ticket file; the
  ignored test `live_corpus_loads_and_counts_sum` loads the repository's own tickets.
