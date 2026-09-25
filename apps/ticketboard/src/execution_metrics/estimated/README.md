# Token estimates

The [ticketboard](/documentation_v2/glossary.md#ticketboard)'s reading of the token estimates in
`.ai/tickets/estimates/`, the counts reconstructed for shipped
[tickets](/documentation_v2/glossary.md#ticket) that have no run receipt: each file checked, summed
per class and per domain in a model of its own, and projected into the estimated rows of the ticket
details.

## Contents

```text
apps/ticketboard/src/execution_metrics/estimated/
├── aggregation.rs        `build_state`, the per-class and per-domain sums over the corpus; `sort_rows`
├── detail_projection.rs  `stamp_cell` and `tokens_cell`, the estimated cells of the ticket details
├── mod.rs                the module tree; holds the imports the children share and re-exports all five
├── models.rs             `EstimateFile`, `ValidEstimate`, `EstimatedTokens`, `EstimatesState`, the sorts
├── services.rs           `load_raw`, which scans the folder, and the panel's fixed texts and glyph
├── tests/                unit tests for sums, buckets, error rows, stamps, sorts and the separation rule
└── validation.rs         `validate_file`, the per-source estimate check, and `cohort_key_str`
```

## How it works

Loading runs in two steps. `load_raw(repo_root)` runs on the application's loading thread: the
folder must be flat, so a subfolder is an error row, and each file becomes a `ValidEstimate` or an
`ErrorRow` holding its path and the reason verbatim. `build_state(raw, corpus)` runs when the board
is built, since it needs the loaded corpus: it keys each estimate by its ticket's class and scope
domain, with "(no ticket file)", "(no class)" and "(program)" as explicit buckets, fills `by_id`
for the ticket details, and builds the headline strip. A missing or empty folder is
`EstimatesState::NoEstimates`.

A file is accepted when its name stem equals its `id` and `validate_file` passes it: the `id`
matches the ticket pattern, `generated_at` is a whole-second UTC stamp and `factor` is at least 1.
A `diff_loc` file carries `loc_changed` and at least one commit in `derived_from_shas` and no cohort
fields, and its `tokens_estimated` equals `loc_changed` times `factor`; a `cohort_median` file
carries a `cohort` key and a `cohort_size` of at least 1 and no `diff_loc` fields. It does not check
that `factor` equals the engine's `TOKENS_PER_LOC`: each file shows the factor it used.

In the ticket details, `stamp_cell` shows a lifecycle stamp listed in the ticket's `estimated`
field with `ESTIMATE_GLYPH` (`~`) and the ticket's `estimate_note` as its tooltip, verbatim, and a
listed stamp with no value as "— (estimated absent)". `tokens_cell` adds a "tokens (estimated)" row
only when `tokens` is listed, and says so when no valid estimate file backs it.

## Boundaries

- Depends on: `crate::execution_metrics::measured` (`ErrorRow`, `format_tokens`,
  `valid_ticket_id`, `valid_git_sha`); `crate::ticket_registry::models` (`Corpus`, and
  `projection` for a ticket's class); `ticket_engine` (`Ticket`, `repository::ESTIMATES_DIR`,
  `validate_rfc3339_utc`); the `serde` and `serde_json` crates; `std::fs`.
- Used by: `crate::application` (`background_loading.rs` calls `load_raw`, `workspace_state.rs`
  calls `build_state`, `action_dispatch.rs` calls `sort_rows`); `crate::ticket_browser`
  (`apps/ticketboard/src/ticket_browser/models/view.rs`, and
  `apps/ticketboard/src/ticket_browser/ui/detail_panel/metadata.rs` and `cells.rs`, which call
  `stamp_cell` and `tokens_cell` and draw `ESTIMATE_GLYPH` and `ABSENT_ESTIMATED_MARKER`);
  `crate::execution_metrics::ui`, `models` and `events`.
- Rules:
  - estimated totals are the `EstimatedTokens` type and live in their own row, totals and model
    types, so no code path adds them to a measured total without an explicit unwrap
    (`the_law_no_code_path_combines_measured_and_estimated` in `tests/estimated.rs`);
  - `validate_file` is a copy of `validate_estimate` in
    `tools_v2/ticket-engine/src/metrics/estimates/model.rs` plus the patterns of
    `.ai/tickets/estimates.schema.json`, kept by hand
    (`checker_mirror_rules_each_produce_a_named_error_row`);
  - `ESTIMATE_GLYPH` equals the scope breadcrumb's `SCOPE_ESTIMATED_GLYPH`
    (`estimate_glyph_matches_the_scope_glyph`);
  - no egui type appears here (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`); the ignored test
    `live_estimates_load_without_error_rows` reads the repository's own estimates.
