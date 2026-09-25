# Language ban and file length gates

The repository checks that keep the codebase in Rust and its files small: `no-shell` and
`no-python` (one hard-zero ban on tracked shell, Make, Python and Node scripts), `no-node` (Node
only as the enfusion-mcp runtime) and `file-length` (500 lines for production Rust, 1000 for
tests).

## Contents

```text
tools_v2/xtask/src/verifications/language_bans/
├── mod.rs                   the module tree
├── node_and_file_limits/    the file-length walk, the no-node checks and the font table generator
├── node_and_file_limits.rs  the file-length limits and roots, the no-node scan subjects; re-exports the entries
├── python_scripts.rs        `verify no-python`: the same ban walk as `no-shell`, under its own name
├── shell_scripts.rs         `verify no-shell`: the tracked-language ban table, shebangs, `python3` calls
└── tests/                   unit tests for the ban walk, shebang parsing and fixture checkouts
```

## How it works

Each gate is a function the `verify` command group calls with no arguments; each finds the
checkout with `git rev-parse --show-toplevel` and returns its exit code.

`verify no-shell` and `verify no-python` run one walk over `git ls-files -z` and differ only in
their closing line. A tracked path fails when:

- its extension or base name is in `TRACKED_LANGUAGE_BANS`: `sh`, `bash`, `zsh`, `ksh`, `fish`,
  `bat`, `ps1`, `py`, `mjs`, `cjs`, `mk`, and `GNUmakefile`, `makefile`, `Makefile`;
- its first line is a shebang whose interpreter, after `env` and its flags, is a shell (`SHELLS`)
  or Python (`PYTHONS`); `#![allow(...)]` is not a shebang;
- `python3` stands in command position in its first 512 KiB (`SCAN_CAP`). In a `.rs` file only
  the first token of a line counts; `.md` files and binary files skip this scan, and `#` and `//`
  comment lines never count.

There is no inventory or allowlist, and no path prefix is skipped: `apps/mod/` is scanned like
everything else, and [Enfusion](/documentation_v2/glossary.md#enfusion) `.c` sources pass because `.c` is not in the table. A failed
`git ls-files`, an empty listing or an unreadable tracked path fails the gate rather than passing
it.

`node_and_file_limits/` holds the `no-node` and `file-length` bodies; its README gives their roots
and rules.

## Public surface

- `shell_scripts::verify_no_shell`, `python_scripts::verify_no_python`,
  `node_and_file_limits::verify_no_node` and `node_and_file_limits::verify_file_length`: the
  entries of `cargo xtask verify no-shell`, `no-python`, `no-node` and `file-length`, and of the
  matching rows of the `ci` task table.
- `node_and_file_limits::gen_font_table`: the body of `cargo xtask gen font-table`.

Exit codes: 0 clean; 1 a banned path, an over-long file or a walk that examined nothing; 2 when
`file-length` could not read a root or a file.

## Boundaries

- Depends on: `git`; `verification_core` (`scan`, `Verdict`, `NotRun`) for the file-length walk;
  `anyhow`.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs` and
    `tools_v2/xtask/src/commands/generate/dispatch.rs`;
  - `tools_v2/xtask/src/commands/ci/task_definitions.rs`, whose `verify-no-python`,
    `verify-no-node`, `verify-no-shell` and `verify-coding-standards` rows run these in process,
    and `ci-local`, which runs those rows;
  - the `language-gates` job of `.github/workflows/ci.yml`;
  - the platform [wave](/documentation_v2/glossary.md#wave) gate
    (`tools_v2/xtask/src/commands/platform/wave_execution/gate/gate_dispatch.rs`), which runs
    `verify no-python`, `verify no-node` and `verify no-shell`.
- Rules:
  - The ban widens only by adding a row to `TRACKED_LANGUAGE_BANS`, never by a path exception
    (`planted_sh_fails`, `makefile_is_banned` and `leftover_py_file_fails` in
    `tests/python_scripts/tests.rs`).
  - A Rust inner attribute is not a shebang, and a commented `python3` is not a call
    (`does_not_sweep_in_rust_inner_attributes` and
    `python3_command_position_ignores_comments` in `tests/shell_scripts/tests.rs`).
  - The file-length limits hold with no exemption file
    (`allowlist_file_must_not_exist` in `tools_v2/xtask/src/tests/node_free_tests.rs`).

## Related documentation

- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the language and
  size rules these gates enforce.
