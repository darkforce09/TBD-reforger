# Measured run receipts

The [ticketboard](/documentation_v2/glossary.md#ticketboard)'s reading of the measured run receipts
under `.ai/tickets/metrics/<id>/`: each file checked by the same rules `cargo xtask ticket check`
applies, then summed per [ticket](/documentation_v2/glossary.md#ticket), per agent and in total,
with every malformed file named instead of counted.

## Contents

```text
apps/ticketboard/src/execution_metrics/measured/
├── aggregation.rs  `build_model` sums runs per ticket, per agent and overall; `sort_rows`
├── formatting.rs   `format_tokens` (thousands commas) and `format_elapsed` (`45s`, `1h 02m 03s`)
├── mod.rs          the module tree; holds the imports the children share and re-exports all four
├── models.rs       `RunReceipt`, `TokensConsumed`, `ErrorRow`, `MeasuredRow`, `MetricsState`, the sorts
├── services.rs     `load_metrics`, the receipt check, and the id and commit patterns `estimated` reuses
└── tests/          unit tests for the empty states, hand-computed sums, error rows, instants and sorts
```

## How it works

`load_metrics(repo_root)` runs on the application's loading thread. A missing or file-less
receipts folder (`ticket_engine::repository::METRICS_DIR`) is the explicit
`MetricsState::NoReceipts`, never a table of zeros. Otherwise it walks the tree depth first in
name order, and every file becomes either a validated run or an `ErrorRow` holding its
repository-relative path and the reason verbatim; an unreadable folder is an error row too.

A receipt is valid when it parses as `RunReceipt` with no unknown field; its `id` matches
`^T-[0-9]+([.][0-9]+)*$` and names the folder it sits in; `agent` is non-empty; `outcome`, when
present, is non-empty; `git_sha`, when present, is 7 to 40 lowercase hex characters; `started` and
`finished` pass `ticket_engine::validate_rfc3339_utc` and `finished` is not before `started`; and
`tokens_consumed.total` equals `input + output + cache_read + cache_write`, with `reasoning` never
added. Elapsed time is `finished` minus `started` in whole seconds, and a run without `finished`
counts as unfinished, never as zero seconds.

`build_model` keys the valid runs by ticket and by agent and totals them in `Grand`. The first
start and the last finish compare parsed instants, not strings, and every display string, the
headline strip included, is built at load time. Both tables start sorted by tokens, descending; a
header click on the same column flips the direction and a new column starts descending, with ties
broken by key name.

## Boundaries

- Depends on: `ticket_engine::repository::METRICS_DIR` and `ticket_engine::validate_rfc3339_utc`;
  the `serde`, `serde_json` and `time` crates; `std::fs`.
- Used by: `crate::application` (`background_loading.rs` calls `load_metrics`;
  `action_dispatch.rs` calls `sort_rows`; `mod.rs` and `events.rs` hold the state and the sort
  types); `crate::execution_metrics::estimated`, which reuses `ErrorRow`, `format_tokens`,
  `valid_ticket_id` and `valid_git_sha`; `crate::execution_metrics::ui`, `models` and `events`.
- Rules:
  - the receipt check is a copy of `validate_record` in
    `tools_v2/ticket-engine/src/metrics/model.rs` plus the patterns of
    `.ai/tickets/metrics.schema.json`, kept by hand
    (`checker_mirror_rules_each_produce_a_named_error_row`,
    `unknown_field_is_an_error_row_mirroring_the_schema`, `ticket_id_and_sha_pattern_mirrors` in
    `tests/measured.rs`);
  - a malformed file is an error row and never part of a sum
    (`bad_sum_receipt_is_a_named_error_row_excluded_from_sums`,
    `missing_tokens_consumed_is_a_named_error_row_never_zero`);
  - an unfinished run counts in runs and unfinished, never in elapsed
    (`unfinished_run_counts_in_runs_and_unfinished_never_in_elapsed`);
  - no egui type appears here (`dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`).
