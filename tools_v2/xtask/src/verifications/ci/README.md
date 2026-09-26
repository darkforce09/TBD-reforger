# CI verifications

Two gates that keep continuous integration honest: `cargo xtask verify ci-shell` keeps the logic
out of the GitHub workflow YAML, so every `run:` step is a `cargo xtask` call or one of a few
setup commands, and `cargo xtask verify ci-schema-parity` keeps the CI schema job, the CI task
table and the [wave](/documentation_v2/glossary/n_to_z.md#wave) gate wired to the full set of schema and
verification steps.

## Contents

```text
tools_v2/xtask/src/verifications/ci/
├── mod.rs                   the module tree
├── schema_parity/           the schema parity entry point, its pins and the YAML comment stripper
├── schema_parity.rs         the ci-schema-parity gate: what it pins and the pinned spellings
├── tests/                   unit tests for schema parity and both workflow shell modules
├── workflow_shell.rs        the ci-shell gate: walks every workflow's jobs and steps as parsed YAML
└── workflow_shell_rules.rs  the rules for one `run:` script, one `shell:` and one `uses:` value
```

## How it works

### ci-shell

`workflow_shell.rs` lists the `.yml` and `.yaml` files directly in `.github/workflows`, parses
each with `serde_norway`, and walks `jobs.*.steps[*]`; `defaults.run` is never a step. Each step
is judged by `workflow_shell_rules.rs`:

- a `uses:` value must be one of `ALLOWED_USES` (`actions/checkout@v7`,
  `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`,
  `actions/upload-artifact@v4`), and a job without steps passes only as a pinned job-level
  `uses:`;
- a `shell:` must not name bash, sh or python, and one step must not hold both `uses:` and `run:`;
- a `run:` script holds one to three logical lines (backslash continuations joined, blank and
  comment lines dropped), none with a heredoc, `$(`, backticks, `&&`, `&`, `||`, a pipe, `;`, a
  redirection, `set -`, or a leading `if`, `for`, `while` or `case`;
- each line starts `cargo xtask`, or is `rustup target add` or `rustup component add`,
  `cargo install` with `--version` or `--vers`, `git lfs pull`, `docker compose`,
  `podman compose` or `docker-compose`.

A missing workflows folder or an unreadable file is "did not run"; a YAML parse error, a missing
or empty `jobs:` or `steps:`, and a run in which no check ran are failures. The shared report
exits 0, 1 or 2.

### ci-schema-parity

`schema_parity/source_audit.rs` strips comments, then pins:

| Subject | Pin |
|---|---|
| the `schema` job of `.github/workflows/ci.yml` | one `run:` is exactly `cargo xtask ci ci-local-schema`, or the same command spelled `cargo run -q -p xtask -- ci ci-local-schema`, once whitespace is normalised |
| the `ci-local-schema` task | its steps include `schema-validate` and `verify-citations` |
| the `verify-mission-rest-size-limits` task | its step is `cargo xtask verify mission-rest-size-limits` |
| the `ci-local` task | it calls `cargo xtask verify ci-schema-parity` in process, and no `verify-ci-schema-parity` task exists |
| `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs` and its two linked files | `VERIFY_STEPS` holds the mission REST size and CI schema parity rows, and both `gate_slice` and `cmd_gate` loop over it and run `cargo run -q -p xtask -- verify <name>` |

The task pins read the CI task table in process, the same `TASKS` that `cargo xtask ci` runs, so
a hollowed task fails here. The gate prints `ci-schema-parity: PASS` and exits 0, or prints each
`FAIL:` line and exits 1, a missing or unreadable input included.

## Public surface

- `workflow_shell::verify_ci_shell` and `run_on_workflows_dir`: the ci-shell gate over the
  repository, or over any folder of workflow files.
- `schema_parity::verify_ci_schema_parity`: the parity gate, taking the repository root.

## Boundaries

- Depends on: `verification-core` (`Report`, `Verdict`, `NotRun`); `serde_norway` for the YAML;
  the `regex` crate; the CI task table in `tools_v2/xtask/src/commands/ci/`; and
  `tools_v2/xtask/src/verifications/architecture/wave_gate_sources.rs`.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify ci-shell` and
    `cargo xtask verify ci-schema-parity`;
  - `tools_v2/xtask/src/commands/ci/task_definitions.rs`, whose `verify-ci-shell` row runs the
    ci-shell gate and whose `ci-local` row runs the parity gate directly;
  - the wave gate's `VERIFY_STEPS` (`CI schema parity`), and in `.github/workflows/ci.yml` the
    `language-gates` job (`verify ci-shell`) and the `mod-gates-hosted` job
    (`verify ci-schema-parity`).
- Rules:
  - the gates parse or read the real subjects rather than grepping text:
    `production_parser_is_the_fixture_parser` holds that the tests run the production walker,
    and `the_live_task_table_satisfies_the_recipe_pins` reads the live `TASKS`;
  - an empty input never passes (`empty_workflows_dir_is_red`,
    `defaults_run_only_no_steps_is_red`, `unreadable_yaml_is_fail_closed`);
  - the parity gate stays off the dispatch table it polices
    (`this_gate_stays_off_the_dispatch_table_it_polices`).
