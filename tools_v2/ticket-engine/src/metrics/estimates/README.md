# Token estimates

Token counts reconstructed for shipped [tickets](/documentation_v2/glossary/n_to_z.md#ticket) that have no
measured run receipt: from the lines their commits changed where commits exist, from the median of
similar tickets otherwise. Estimates live in `.ai/tickets/estimates/`, apart from the measured
receipts, so one can never pass for the other.

## Contents

```text
tools_v2/ticket-engine/src/metrics/estimates/
├── cohorts.rs       the cohort ladder and the median that fill in tickets without changed lines
├── git_changes.rs   `collect_numstat`, `parse_numstat`: changed lines per commit, minus bookkeeping
├── incremental.rs   `plan_estimate_for_id` and `derivation_shas`: one ticket's estimate for `stamp-sha`
├── mod.rs           the module tree; re-exports the model, the passes and the check
├── model.rs         `EstimateRecord`, `CohortKey`, `TOKENS_PER_LOC` and `validate_estimate`
├── planning.rs      `plan_estimates`: the pure pass over every shipped ticket
├── storage.rs       reads and writes `estimates/<id>.json`, and `run_estimates`, the writing pass
├── tests/           unit tests for the factor pin, the exclusions, the cohorts and the check
└── verification.rs  `check_as_errors`: the estimate rules `ticket check` enforces
```

## How it works

An estimate is one of two sources, each with its own fields:

| Source | When | Tokens | Fields only it carries |
|---|---|---|---|
| `diff_loc` | the ticket's subject commits changed at least one counted line | changed lines × `TOKENS_PER_LOC` (150) | `loc_changed`, `derived_from_shas` |
| `cohort_median` | no subject commit, or commits that changed only excluded paths | the median of the `diff_loc` tickets in its cohort | `cohort`, `cohort_size` |

- Lines: `collect_numstat` runs `git log --numstat --pretty=%H` once and counts, per commit, the
  lines of every path except the bookkeeping ones `is_excluded_path` names: the `.ai/` tree, the
  retired queue views (`RETIRED_QUEUE_VIEW_PREFIX`, `.md` files only) and every `Cargo.lock`. The
  subject commits come from `mine_subjects` in `tools_v2/ticket-engine/src/cli/shipping/`.
- Cohorts: `cohort_for` walks from the ticket's most specific key (class, domain, layer) through
  class and domain, then class, to all `diff_loc` tickets, and takes the first level with three or
  more members; the last level takes whatever it has. The key recorded is the level used.
- `plan_estimates` skips tickets that are not shipped, have a receipt or already have an
  estimate, plans `diff_loc` records first and `cohort_median` records second, and validates each
  before returning it. `run_estimates` writes the planned files, adds `tokens` to each ticket's
  `estimated` list and re-checks the tree. `plan_estimate_for_id` does the same for one ticket
  from an explicit commit set, for `cargo xtask ticket stamp-sha`.
- Files are pretty JSON with keys in alphabetical order, the declaration order of
  `EstimateRecord`.

## Boundaries

- Depends on: `crate::Corpus` and the ticket model; `crate::cli::commit_subjects::SubjectCommit`;
  `crate::repository` (`ESTIMATES_DIR`, `ESTIMATES_SCHEMA`, and `documentation`'s
  `TOKEN_ESTIMATE_FACTOR_DOC`, `NUMSTAT_EXCLUDED_PREFIXES` and `RETIRED_QUEUE_VIEW_PREFIX`); the
  receipt lookup in `crate::metrics`; `.ai/tickets/estimates.schema.json`, checked through
  `jsonschema`; `git`.
- Used by: `cmd_stamp_sha` in `tools_v2/ticket-engine/src/cli/shipping.rs`; `ticket check`
  through `tools_v2/ticket-engine/src/validation/runner.rs`, and its `--strict` counters.
  `run_estimates` has no caller outside the tests. The ticketboard keeps its own copy of
  `validate_estimate` (`apps/ticketboard/src/execution_metrics/estimated/validation.rs`).
- Rules:
  - `TOKENS_PER_LOC` is quoted verbatim in the factor document, which also documents every
    excluded path (`factor_constant_is_pinned_in_the_doc` in `tests/estimate_provenance_tests.rs`);
  - `ticket check` refuses an estimate that fails the schema or `validate_estimate`, whose file
    stem differs from its `id`, that sits in a subfolder, whose ticket is not shipped, whose
    `factor` is not `TOKENS_PER_LOC`, that shares a ticket with a run receipt, or whose presence
    disagrees with `tokens` in the ticket's `estimated` list (`business_rules_red` and
    `mutual_exclusion_and_marker_coherence`);
  - a pass never writes a tree its own check refuses (`scratch_generator_cohorts_fallthrough_and_idempotence`).

## Related documentation

- [Token estimate factor](/documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md) — the
  measurement behind `TOKENS_PER_LOC` and the excluded paths.
