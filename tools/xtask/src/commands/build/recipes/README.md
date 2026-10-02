# Build recipe execution

The two halves of `cargo xtask mk` that `tools/xtask/src/commands/build/recipes.rs` declares as
submodules: the entry point that picks a target, and the recipes with the runner that executes
them. The command itself is described in the
[build commands README](/tools/xtask/src/commands/build/README.md).

## Contents

```text
tools/xtask/src/commands/build/recipes/
├── execution.rs   `run`, which prints or runs a target's recipe, and `recipe_lines`
└── shell_word.rs  every recipe as a list of `Step`s, the step runner, and the `rust-ci` composite
```

## How it works

`run` in `execution.rs` takes the first argument that does not start with `-` as the target;
`--dry-run` or `-n` anywhere makes it print the recipe instead of running it. With no target it
prints the usage line and `TARGETS`, exiting 0 with `--list` and 2 otherwise; an unknown target
exits 2. The three targets that compute rather than build (`print-cargo-target-dir`,
`verify-cargo-target`, `reclaim-target-ci`) call into
`tools/xtask/src/core/cargo_target_directory.rs`; `rust-ci` runs its composite; every other
target gets its step list from a function in `shell_word.rs`. `recipe_lines` returns the lines a
recipe target runs, as `--dry-run` prints them, which the recipe tests scan.

`run_steps` in `shell_word.rs` runs a list in order and stops at the first non-zero exit. For
each step it:

1. resolves the target directory: the step's own `CARGO_TARGET_DIR` (only `rust-api` sets one,
   `target/dev-api` in the checkout) or the shared pin from `cargo_target_directory.rs`;
2. for `cargo` and `trunk`, refuses with exit 1 when `abi_guard` finds that directory stamped by
   another glibc;
3. prints the step's `echo()` line, rendered from the same fields that run;
4. spawns it with inherited stdio, in the step's directory under the checkout, with the pin
   exported.

A tool that is not installed exits 127 with a did-not-run verdict; a child killed by a signal exits
128 plus the signal number with a verdict that names the signal; otherwise the child's own code
passes through. `rust_ci` runs `rust-fmt`, `rust-clippy`, `rust-build` and `wasm-ci` in that
order, stopping at the first failure, then prints `cargo xtask db test-it` and calls the database
lane's complete suite in process. That lane resolves the container runtime, creates a fresh
database for the run and drops it and every per-binary database derived from it after every
outcome, so no recipe here names a container runtime or cleans a database itself.

## Boundaries

- Depends on: `Step`, `TARGETS` and the folder constants in `recipes.rs`;
  `tools/xtask/src/core/cargo_target_directory.rs` for the pin, the ABI guard and the three
  computed targets; `verification_core` for verdicts;
  `tools/xtask/src/commands/db/operations/test_it.rs` for `rust-ci`'s integration tests.
- Used by: `recipes.rs`, which re-exports the recipe functions and `run`; the recipe tests in
  `tools/xtask/src/commands/build/tests/recipes.rs`.
- Rules: a step's printed line and its execution come from the same fields, never two copies
  (`echo_matches_make` pins the echoes); `leptos-gates` builds the app once
  (`leptos_gates_does_not_double_build`); no step detaches from the terminal, so Ctrl-C reaches
  `trunk serve`.
