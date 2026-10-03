# Ticket metrics

The `ticket_metrics` crate: what running a [ticket](/documentation/glossary/n_to_z.md#ticket)
cost. It writes, stamps, checks and summarises the run receipts an agent run leaves under
`.ai/tickets/metrics/<id>/`, and reconstructs token estimates under `.ai/tickets/estimates/` for
shipped tickets that have no receipt. `ticket_registry` and the `ticket` and `platform` command
groups of xtask call it.

## Contents

```text
tools/tickets/ticket_metrics/
├── Cargo.toml  the `ticket_metrics` library package: `ticket_model`, `jsonschema`, `time`, layout tier 3
├── src/        the receipt model, writer, land stamp, summary and check, and the token estimates
└── tests/      the recorded agent CLI outputs the token parser is pinned against
```

## How it works

```text
platform slice-run ──parse_tokens_from_cli_json──► RunRecord ──write_run_file──► metrics/<id>/<started>-<sha>.json
platform wave land ──land_receipt_refusal, stamp_land──► newest run file: outcome, git_sha, finished
ticket metrics     ──cmd_metrics──► one line per run, or per agent
ticket check       ──check_as_errors, estimates::check_as_errors──► one error per bad file or ticket
ticket stamp-sha   ──estimates::plan_estimate_for_id, write_estimate_file──► estimates/<id>.json
```

Two trees with separate files, schemas and checks hold the two kinds of number. A receipt under
`.ai/tickets/metrics/<id>/` is a measurement: the token counts the agent CLI reported, which
`src/token_usage.rs` parses and `src/model.rs` checks for the four-way sum. An estimate under
`.ai/tickets/estimates/<id>.json` is a reconstruction from git history, in `src/estimates/`: the
lines the ticket's commits changed times `TOKENS_PER_LOC`, or the median of similar tickets. A
ticket has one or the other, never both, so an estimate can never pass for a measurement.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p ticket_metrics      # the receipt and estimate suites; one test reads this checkout's git log
cargo xtask ticket metrics        # the run receipts of this checkout, one line per run
cargo xtask ticket check          # the registry check, including the receipt and estimate rules
```

`cargo xtask ticket metrics --by agent` prints one line per agent instead.

## Configuration

No feature and no environment variable. Every function takes the checkout root and reads the
paths `repository_layout` names: `.ai/tickets/metrics/<id>/`, `.ai/tickets/metrics.schema.json`,
`.ai/tickets/estimates/` and `.ai/tickets/estimates.schema.json`. `collect_numstat` runs `git`
from `PATH`.

## Public surface

- At the crate root: `RunRecord`, `TokensConsumed`, `validate_record`, `elapsed_sec`,
  `metrics_root`, `parse_tokens_from_cli_json` and `write_run_file` for `platform slice-run`;
  `has_receipt`, `missing_receipts`, `land_receipt_refusal`, `stamp_land` and `stamp_land_at` for
  the platform wave landing; `latest_run_file`; `cmd_metrics` and `summarize_by_agent` for
  `ticket metrics`; `check_as_errors` for `ticket check`; `Error` and `Result`.
- `estimates`: `EstimateRecord`, `CohortKey`, `TOKENS_PER_LOC`, `validate_estimate`,
  `estimates_root`, `collect_numstat`, `parse_numstat`, `is_excluded_path`, `plan_estimates`,
  `EstimateReport`, `plan_estimate_for_id`, `derivation_shas`, `load_existing`,
  `write_estimate_file`, `run_estimates` and `check_as_errors`.
- `prelude`: `RunRecord`, `TokensConsumed`, `has_receipt`, `metrics_root` and `write_run_file`.

## Boundaries

- Depends on: `ticket_model` (`TicketId`, the corpus, the subject commits, the factor document
  path and the excluded numstat prefixes), `repository_layout` (the tree and schema paths),
  `time_source` (the RFC 3339 UTC rule and the clock), `process_runner` (the `git log` run),
  `jsonschema`, `time`, `serde_json`, `walkdir` and `thiserror`; the schemas
  `.ai/tickets/metrics.schema.json` and `.ai/tickets/estimates.schema.json`.
- Used by: `ticket_registry` (`ticket check` in `tools/tickets/ticket_registry/src/validation/`,
  `stamp-sha` in `tools/tickets/ticket_registry/src/verbs/shipping.rs`, and the `Metrics`
  variant of its `Error`); xtask's `ticket metrics`
  (`tools/xtask/src/commands/ticket/dispatch.rs`), `platform slice-run`
  (`tools/commands/platform_execution/src/slice_execution.rs`) and the wave landing
  (`tools/commands/platform_execution/src/wave_execution/land/merge_execution.rs`). The xtask test
  `tools/commands/platform_execution/src/tests/slice_execution/tests.rs` replays the fixtures in
  `tests/fixtures/execution_receipts/`. The ticketboard reads both trees with its own copies of
  the record checks (`apps/ticketboard/src/execution_metrics/`).
- Rules: tier 3 of `tools/tickets`, depending on no ticket crate but `ticket_model`
  (`cargo xtask verify crate-tiers`); a receipt total is the four-way sum and a usage block in
  neither recorded dialect fails rather than reading as zero; a ticket never carries both a
  receipt and an estimate; the files under `tests/fixtures/` are recorded data, which the xtask
  prose rules skip (`tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`).

## Related documentation

- [Ticket crates](/tools/tickets/README.md) — the four ticket crates and their tiers.
- [Ticket documentation](/documentation/tools/tickets/README.md) — the documents on the ticket
  registry and its commands.
- [Token estimate factor](/documentation/tools/tickets/token_estimate_factor.md) — the
  measurement behind `TOKENS_PER_LOC` and the excluded paths.
