# Browser filters and scope facets

The filters of the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s browser: per-ticket
facts computed once per load, the composable filter set applied over them, and the scope
dropdowns narrowed from the scope vocabulary and the values the corpus holds.

## Contents

```text
apps/ticketboard/src/ticket_browser/services/
├── filtering.rs     `FilterIndex` of per-ticket facts, and `Filters` with their per-ticket verdicts
├── mod.rs           the module tree
├── scope_facets.rs  `VocabTree` from the scope vocabulary, and `compute`, the narrowed options
└── tests/           unit tests for filter composition and clearing, facet narrowing and fallback
```

## How it works

`FilterIndex::build` records, for each ticket, its lowercase id and explicit parent, executor
label, kind, status, scope (domain, layer, component, surfaces; none on a program), class, and a
lowercase haystack of id, title and summary, plus the sorted list of executors. `Filters` holds the
text, executor, kind, status toggles, parent id, the four scope facets and the class. All of them
combine with AND; a status set with no toggle on means every status, and `clear` resets every
field. The parent filter keeps the id itself, its dotted descendants (`T-915` keeps `T-915.2`,
never `T-9150`) and tickets whose `parent` names it, ignoring case. `apply` returns a verdict per
ticket and the match count.

`VocabTree::load` reads `.ai/tickets/scope-vocab.toml` as a tree of domain, layer, component and
surfaces. Any failure (a missing file, a read error or a wrong shape) gives `None`, all or nothing,
and the dropdowns fall back to the values the corpus holds. `compute` builds the four option lists,
each narrowed by the selections above it, as the union of the vocabulary and the corpus; a corpus
value the loaded vocabulary does not offer there is marked `vocab_unknown`, for display only. It
then clears every lower selection its list no longer offers, so no hidden selection pins the board
to zero matches.

Both run only when a filter changes or the corpus reloads: `WorkspaceState::refilter` in
`apps/ticketboard/src/application/workspace_state.rs` computes the facets, applies the filters,
and rebuilds the visible board rows and tree rows from the verdicts.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, the `projection` view, `Class`,
  `executor_label`, `column_of`); `ticket_engine` (`StatusName`, `Ticket`,
  `repository::SCOPE_VOCAB`); the `toml` crate.
- Used by: `crate::ticket_browser::ui::filter_bar`; `crate::application` (`mod.rs`,
  `workspace_state.rs`, and `background_loading.rs`, which loads the vocabulary on the worker
  thread).
- Rules:
  - filters change the projection, never the registry, and clearing restores the full count
    (`clear_restores_the_full_measured_count`, `filters_compose_as_intersection` in
    `tests/filtering.rs`);
  - the vocabulary marks and narrows but never rejects a value; `cargo xtask ticket check` stays
    the validation authority (`corpus_strays_are_offered_and_marked`,
    `broken_or_missing_vocab_is_none_never_a_crash`,
    `stale_lower_selections_are_cleared_top_down` in `tests/scope_facets.rs`).
