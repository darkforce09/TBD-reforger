# Ticket metrics source

What running a [ticket](/documentation/glossary/n_to_z.md#ticket) cost: the run receipts an agent
run leaves under `.ai/tickets/metrics/<id>/`, with their token counts, timestamps and landing
commit, and, in `estimates/`, the token counts reconstructed for shipped tickets without a
receipt. The two trees have separate files, schemas and checks, so an estimate can never pass for
a measurement.

## Contents

```text
tools/tickets/ticket_metrics/src/
├── error.rs         `Error` and `Result`, and the crate-private `ResultExt` that adds a context line
├── estimates/       token estimates from changed lines or cohort medians, and their check
├── lib.rs           the crate root: module header, `mod` lines and the re-exports
├── model.rs         `RunRecord` and `TokensConsumed`, `validate_record`, `elapsed_sec` and `metrics_root`
├── prelude.rs       the receipt model, `has_receipt`, `metrics_root` and `write_run_file` for glob import
├── receipts.rs      writes, finds and stamps run files, and the landing gate on missing receipts
├── summary.rs       `cmd_metrics` and `summarize_by_agent`: the `ticket metrics` report
├── tests/           unit tests for both dialects, the token sum, file naming, stamping and the gate
├── token_usage.rs   `parse_tokens_from_cli_json`: token counts from an agent CLI's final JSON
└── verification.rs  `check_as_errors`: every receipt against its schema and invariants
```

## How it works

```text
platform slice-run ──parse_tokens_from_cli_json──► RunRecord ──write_run_file──► metrics/<id>/<started>-<sha>.json
platform wave land ──stamp_land──► newest run file: outcome = landed, git_sha, finished
ticket check ──check_as_errors──► every file: metrics.schema.json + validate_record + folder name
ticket metrics ──cmd_metrics──► runs, elapsed and token sums, optionally per agent
```

- A receipt holds `id`, `agent`, `started` and `tokens_consumed`, with `finished`, `outcome` and
  `git_sha` stamped later. `tokens_consumed.total` is always `input + output + cache_read +
  cache_write`; `reasoning` is recorded beside it and never added. Elapsed time is computed from
  `finished` minus `started`, never stored. Timestamps follow the same UTC rule as the ticket
  stamps (`time_source::validate_rfc3339_utc`).
- `parse_tokens_from_cli_json` reads the two recorded dialects, the Cursor agent's
  (`usage.inputTokens`, …) and Claude's (`usage.input_tokens`, …), and fails on anything else
  rather than recording zeros. The recorded outputs sit in the crate's
  `tests/fixtures/execution_receipts/`.
- A run file is named by its compact start time and the first 12 characters of its commit
  (`nosha` when there is none), with `-1`, `-2`, … added when a name is taken, so two runs never
  share a file; the newest one is chosen by `started`, then by name length and name.
- `land_receipt_refusal` stops a landing whose tickets have no receipt unless the land is marked
  `--bookkeeping`; `stamp_land` refuses when there is no run file to stamp and never writes token
  counts.
- `cmd_metrics` and `summarize_by_agent` stop at the first file that cannot be read, parsed or
  validated, naming it, so a broken receipt never reads as zero tokens.
- Every failure is an `Error`; a step that adds context wraps the cause as an `Error::Context`
  whose display is its own line, so a printed chain shows `context: cause` once.

## Boundaries

- Depends on: `repository_layout` (`METRICS_DIR`, `METRICS_SCHEMA`); `time_source` for the UTC
  rule and the clock, and the `time` crate for the arithmetic; `ticket_model` (`TicketId`, the
  id order of the report, `error_chain_text`); `jsonschema` against
  `.ai/tickets/metrics.schema.json`; `walkdir` for the receipt tree.
- Used by: `ticket_registry`'s `ticket check` (`tools/tickets/ticket_registry/src/validation/`)
  and `stamp-sha` (`tools/tickets/ticket_registry/src/verbs/shipping.rs`); xtask's
  `platform slice-run` (`tools/commands/platform_execution/src/slice_execution.rs`), the platform
  wave landing (`tools/commands/platform_execution/src/wave_execution/land/merge_execution.rs`) and
  `ticket metrics` (`tools/xtask/src/commands/ticket/dispatch.rs`). The ticketboard reads the
  receipt tree itself, with its own copy of `validate_record`
  (`tools/tickets/ticketboard_model/src/execution_metrics/measured/`).
- Rules:
  - a total that is not the four-way sum is refused (`total_sum_invariant_is_enforced`), and a
    usage block in neither dialect fails instead of reading as zero
    (`missing_usage_fails_closed_never_zero`);
  - two runs started in the same second get two files (`two_runs_in_one_second_yield_two_files`);
  - stamping a landing touches only the metrics tree, never a ticket file
    (`land_stamp_two_tickets_touches_only_metrics_never_ticket_tomls`);
  - a receipt that fails the schema or the invariants is an error in `ticket check`, named by file
    (`malformed_started_missing_id_missing_tokens_and_bad_sum_are_red`).

## Related documentation

- [Token estimate factor](/documentation/tools/tickets/token_estimate_factor.md) — the
  measurement behind the estimates.
