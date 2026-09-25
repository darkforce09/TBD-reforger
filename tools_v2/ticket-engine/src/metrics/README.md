# Execution metrics

What running a [ticket](/documentation_v2/glossary.md#ticket) cost: the run receipts an agent run
leaves under `.ai/tickets/metrics/<id>/`, with their token counts, timestamps and landing commit,
and, in `estimates/`, the token counts reconstructed for shipped tickets without a receipt. The
two trees have separate files, schemas and checks, so an estimate can never pass for a
measurement.

## Contents

```text
tools_v2/ticket-engine/src/metrics/
├── estimates/       token estimates from changed lines or cohort medians, and their check
├── mod.rs           the module tree; re-exports the receipt model, the receipt helpers and the check
├── model.rs         `RunRecord` and `TokensConsumed`, `validate_record` and `elapsed_sec`
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
ticket check ──check_as_errors──► every file: metrics.schema.json + validate_record
ticket metrics ──cmd_metrics──► runs, elapsed and token sums, optionally per agent
```

- A receipt holds `id`, `agent`, `started` and `tokens_consumed`, with `finished`, `outcome` and
  `git_sha` stamped later. `tokens_consumed.total` is always `input + output + cache_read +
  cache_write`; `reasoning` is recorded beside it and never added. Elapsed time is computed from
  `finished` minus `started`, never stored. Timestamps follow the same UTC rule as the ticket
  stamps (`crate::validate_rfc3339_utc`).
- `parse_tokens_from_cli_json` reads the two recorded dialects, the Cursor agent's
  (`usage.inputTokens`, …) and Claude's (`usage.input_tokens`, …), and fails on anything else
  rather than recording zeros.
- A run file is named by its compact start time and the first 12 characters of its commit, with
  `-1`, `-2`, … added when a name is taken, so two runs never share a file; the newest one is
  chosen by `started`.
- `land_receipt_refusal` stops a landing whose tickets have no receipt unless the land is marked
  `--bookkeeping`; `stamp_land` refuses when there is no run file to stamp and never writes token
  counts.

## Public surface

- `RunRecord`, `TokensConsumed`, `validate_record`, `parse_tokens_from_cli_json`,
  `write_run_file`: `platform slice-run` (`tools_v2/xtask/src/commands/platform/slice_execution.rs`).
- `missing_receipts`, `land_receipt_refusal`, `stamp_land`: the platform wave landing
  (`tools_v2/xtask/src/commands/platform/wave_execution/land/merge_execution.rs`).
- `cmd_metrics`: `cargo xtask ticket metrics [--by agent]`.
- `check_as_errors` and `has_receipt`: `ticket check` and the estimate rules in this crate;
  `has_receipt` also the platform wave landing.

## Boundaries

- Depends on: `crate::repository` (`METRICS_DIR`, `METRICS_SCHEMA`); `crate::validate_rfc3339_utc`
  and the `time` crate for the arithmetic; `jsonschema` against
  `.ai/tickets/metrics.schema.json`; `walkdir` for the receipt tree.
- Used by: `crate::validation`, `crate::cli` (`stamp-sha`) and `ticket check --strict`'s counters;
  the xtask callers under Public surface. The ticketboard reads the receipt and estimate trees
  itself, through `crate::repository`'s `METRICS_DIR` and `ESTIMATES_DIR`, with its own copies of
  `validate_record` and `validate_estimate` (`apps/ticketboard/src/execution_metrics/`).
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

- [Token estimate factor](/documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md) — the
  measurement behind the estimates.
