# Browser display models

The projections the [ticketboard](/documentation_v2/glossary.md#ticketboard)'s browser paints from,
built once per load so the paint path never sorts or formats: the status board's columns and
cards, the program tree, the order of a ticket's detail sections, and the borrowed view the
application lends the browser.

## Contents

```text
apps/ticketboard/src/ticket_browser/models/
├── detail_sections.rs  `BodyField`, header and body order, section content, quarantine, triage block
├── mod.rs              the module tree
├── program_tree.rs     `TreeModel` from parents and dotted ids, with cycle rescue, and `flatten`
├── status_board.rs     `BoardModel`: eight status columns of precomputed `Card`s, the id-to-index map
├── tests/              unit tests for the board sort and cards, the tree and the section order
└── view.rs             `BrowserView`, the borrowed data the browser paints, and `DraggedTicket`
```

## How it works

- `BoardModel::build` puts every ticket of the corpus in the column of its status, in
  `STATUS_ORDER` (`idea` through `cancelled`), and sorts each column by `order`, tickets without
  one last, then by numeric id (`T-9` before `T-10`, `T-915.2` before `T-915.10`). A `Card` holds
  the id, the title cut to 48 characters, the executor (`claude-code` when unset), the `#order`
  label, the scope breadcrumb of a work ticket, the class, and the `main_goal` as its tooltip when
  it has text. Column headers read `<status> · <count>`. `id_to_index` resolves every id to its
  corpus index.
- `TreeModel::build` hangs each ticket under its explicit `parent` when that resolves, else under
  the ticket its dotted id extends; siblings sort like cards, and titles are cut to 56 characters.
  Tickets caught in a parent loop, which no root reaches, become extra roots rather than vanish.
  `flatten` turns the tree into paint rows: with no filter it follows the manual expansion; with a
  filter it keeps each match and the path to it, opens that path, and dims the ancestors that do
  not match themselves.
- `detail_sections.rs` owns the order of the ten typed body fields (`summary`, `main_goal`,
  `context`, `requirement`, `current_state`, `approach`, `verify`, `acceptance`, `citations`,
  `notes`), each with a one-line definition used as its tooltip. `main_goal` and `summary` form the
  header, shown only when they have text; the body region is the other eight in order, then the
  `migration_legacy` quarantine. A missing or blank field is `SectionContent::Absent`, drawn as
  "—"; list fields are numbered. The quarantine shows 8 lines and collapses the rest, and
  `triage_block` builds the clipboard text: the id, the legacy lines verbatim and an empty
  ten-field skeleton.
- `BrowserView` borrows the corpus, board, tree, flattened rows, column expansion, visible rows,
  selection, comparison and estimates, so rendering can neither reach application jobs nor change
  the registry.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus` and the `projection` view, sort keys,
  labels, breadcrumbs and classes); `crate::execution_metrics::estimated::EstimatesState` for
  `BrowserView`; `ticket_engine` (`StatusName`, `Ticket`).
- Used by: `crate::ticket_browser::ui`; `crate::application` (`mod.rs`, `workspace_state.rs`,
  `feature_views.rs`), which builds the board and tree at load and flattens the tree on every
  filter or expansion change.
- Rules:
  - cards sort by order then numeric id, and ids that do not parse sort last
    (`cards_sort_by_order_then_numeric_id`, `unparsable_ids_sort_last` in `tests/status_board.rs`);
  - an explicit parent beats a dotted id, and a parent loop never drops a ticket
    (`explicit_parent_beats_dotted_prefix`, `parent_cycle_rescued_as_root` in
    `tests/program_tree.rs`);
  - the body starts at `context` and the quarantine comes last
    (`section_order_is_pinned_and_quarantine_is_last` in `tests/detail_sections.rs`);
  - nothing here names egui (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`).
