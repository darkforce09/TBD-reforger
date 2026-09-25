# Ticket validation

`cargo xtask ticket check`: every rule the [ticket](/documentation_v2/glossary.md#ticket) files
and the files derived from them must hold, returned as one ordered list of findings, and the
preflight every ticket mutation runs before it writes.

## Contents

```text
tools_v2/ticket-engine/src/validation/
├── body.rs        word caps, rules between body fields, and new `migration_legacy` fields
├── command.rs     `cmd_check`, `require_check_ok` and its batch-ship variant, and the `--strict` counters
├── constants.rs   the queue header, roadmap markers, target, executor and stream sets, id patterns
├── debt.rs        the title and `main_goal` debt counts against their pins, and the counter lines
├── mod.rs         the module tree; re-exports `check`, `cmd_check`, the preflights and the schema check
├── readiness.rs   the plan gate, non-empty work titles and the ready-tier body fields
├── references.rs  child integrity, spec and plan files on disk, fossil path and stale id scans
├── runner.rs      `check`: runs every rule in a fixed order and collects the findings
├── schema.rs      the registry against `.ai/tickets/schema.json`, then ids, live orders, rows
├── scope.rs       `owns` on open work, `class` on work, a surface on live work with a component
├── shipping.rs    the ship gate and the agreement between `estimated` entries and the stamps
├── tests/         unit tests for each rule, red and green, over scratch trees and the live tree
└── vocabulary.rs  the shape of `.ai/tickets/scope-vocab.toml`
```

## How it works

`check(root, registry, strict)` takes the checkout root and the registry projection and returns
every finding as a string, in this order: the schema, then the row rules, `owns`, class and
surface, the body rules, the `estimated` stamps, parent and child integrity, the wave lock
(`crate::wave_lock::check_as_errors`), the run receipts (`crate::metrics::check_as_errors`), the
estimates (`crate::metrics::estimates::check_as_errors`), the ship gate, the plan gate, work
titles, the ready-tier body, the debt pins, the scope vocabulary, the fossil path guard; then
each row's targets, executor and stream against `constants.rs`, the ids that
`.ai/tickets/corpus-pins.toml` says are never minted, spec and plan files on disk, both roadmap
markers, and the gap-analysis round trip. `strict` adds the stale ticket id scan
(`STALE_TICKET_ID_SCAN_ROOTS`, minus `SCAN_EXEMPT_PREFIXES`) and refuses a priority column in the
gap analysis.

Every rule that needs the typed corpus loads it itself and reports a load failure as a finding; a
rule never passes over input it could not read. The fossil path guard runs `git grep` over the
tracked tree for the archived wave plan names and allows only the readers
`ARCHIVED_WAVE_PLAN_READERS` lists.

`cmd_check`, which `cargo xtask ticket check [--strict]` runs, prints the debt counter lines on
every run and the token and stamp counters under `--strict` (the body rules also print a
`WARNING:` line for each acceptance entry shaped like a command, which never fails the check),
then either `check OK` on stdout or each finding as an `ERROR:` line on stderr and exit 1.
`require_check_ok` is the preflight of every mutation in `crate::cli`: the same check without
`strict`, returning an error instead of exiting. `require_check_ok_deferring_repack`, for `ticket ship --no-repack`, waives the findings
whose text asks for `cargo xtask wave repack`, but never the missing-lock refusal.
`constants` also serves `crate::sync`, which writes the markers and the queue header it defines.

## Boundaries

- Depends on: the registry projection (`crate::registry`) and the typed corpus (`crate::Corpus`);
  the model's caps, pins and predicates; `crate::wave_lock`, `crate::metrics`,
  `crate::corpus_pins` and `crate::sync::gap_analysis`; `crate::repository` and its
  `documentation` paths; `.ai/tickets/schema.json` through `jsonschema`; `git` for the fossil path
  guard.
- Used by: `cargo xtask ticket check` through `tools_v2/xtask/src/commands/ticket/mod.rs`, and
  through it the platform preflight, the platform and mod wave gates and the `language-gates` CI
  job; every mutation in `tools_v2/ticket-engine/src/cli/`; the ticketboard, which runs
  `cargo xtask ticket check --strict` and reads its `check OK` and `ERROR:` lines
  (`apps/ticketboard/src/repository_status/models/check_status.rs`).
- Rules:
  - the committed tree passes the full check (`tip_registry_full_check_ok` in
    `tests/schema_and_integrity_tests.rs`), and every spec and plan it names exists
    (`live_tree_names_only_existing_spec_and_plan_files`);
  - a check that cannot load its input reports the load error
    (`an_unloadable_corpus_reports_the_load_error`);
  - a red check blocks every mutation (`require_check_ok_blocks_invalid_registry`);
  - both debt pins stay equal to the measured counts, in both directions
    (`debt_pin_growth_verdict`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket fields, statuses and files these rules
  hold.
- [Ticket command group](/tools_v2/xtask/src/commands/ticket/README.md) — the `ticket check`
  command and the mutations it guards.
