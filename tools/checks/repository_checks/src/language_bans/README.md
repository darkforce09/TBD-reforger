# Language ban and file length gates

The repository checks that keep the codebase in Rust and its files small: `no-shell` and
`no-python` (one hard-zero ban on tracked shell, Make, Python and Node scripts), `no-node` (Node
only as the enfusion-mcp runtime) and `file-length` (500 lines for a production Rust or Enfusion
script file, 1000 for tests).

## Contents

```text
tools/checks/repository_checks/src/language_bans/
├── mod.rs                   the module tree
├── node_and_file_limits/    the file-length report and the no-node checks
├── node_and_file_limits.rs  the no-node scan subjects; re-exports the entries
├── python_scripts.rs        `verify no-python`: the same ban walk as `no-shell`, under its own name
└── shell_scripts.rs         `verify no-shell`: the tracked-language ban table, shebangs, `python3` calls
```

## How it works

Each gate is a function the `verify` command group calls with no arguments; each finds the
checkout with `repository_root::find_repository_root` and returns its exit code; a working
directory outside a checkout is an error, never an empty root.

`verify no-shell` and `verify no-python` run one walk over `git ls-files -z` and differ only in
their closing line. A tracked path fails when:

- its extension or base name is in `TRACKED_LANGUAGE_BANS`: `sh`, `bash`, `zsh`, `ksh`, `fish`,
  `bat`, `ps1`, `py`, `mjs`, `cjs`, `mk`, and `GNUmakefile`, `makefile`, `Makefile`;
- its first line is a shebang whose interpreter, after `env` and its flags, is a shell (`SHELLS`)
  or Python (`PYTHONS`); `#![allow(...)]` is not a shebang;
- `python3` stands in command position in its first 512 KiB (`SCAN_CAP`). In a `.rs` file only
  the first token of a line counts; `.md` files and binary files skip this scan, and `#` and `//`
  comment lines never count.

There is no inventory or allowlist, and no path prefix is skipped: `mod/` is scanned like
everything else, and [Enfusion](/documentation/glossary/a_to_f.md#enfusion) `.c` sources pass because `.c` is not in the table. A failed
`git ls-files`, an empty listing or an unreadable tracked path fails the gate rather than passing
it.

`node_and_file_limits/` holds the `no-node` and `file-length` bodies; its README gives their
rules. The file-length roots, ceiling and test-file rule live in `repository_laws`.

## Public surface

- `shell_scripts::verify_no_shell`, `python_scripts::verify_no_python`,
  `node_and_file_limits::verify_no_node` and `node_and_file_limits::verify_file_length`: the
  entries of `cargo xtask verify no-shell`, `no-python`, `no-node` and `file-length`, and of the
  matching rows of the `ci` task table.

Exit codes: 0 clean (an over-long file is only a warning); 1 a banned path or a walk that examined
nothing; 2 when `file-length` could not read a root or a file.

## Boundaries

- Depends on: `git`; `verification_core` (`repository_laws::file_length` for the file-length
  scan, `Verdict` and `NotRun` for its refusal); `anyhow`.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs` and
    `tools/commands/schema_tooling/src/generate/dispatch.rs`;
  - `tools/commands/ci_task_catalog/src/task_definitions.rs`, whose `verify-no-python`,
    `verify-no-node`, `verify-no-shell` and `verify-coding-standards` rows run these in process,
    and `ci-local`, which runs those rows;
  - the `language-gates` job of `.github/workflows/ci.yml`;
  - the ticket manager's platform [wave](/documentation/glossary/n_to_z.md#wave) gate
    ([`ticket_manager_execution.toml`](/ticket_manager_execution.toml)), which runs
    `verify no-python`, `verify no-node` and `verify no-shell`.
- Rules:
  - The ban widens only by adding a row to `TRACKED_LANGUAGE_BANS`, never by a path exception.
  - A Rust inner attribute is not a shebang, and a commented `python3` is not a call.
  - The file-length gate is advice: a long production file prints a warning, never a failure.

## Related documentation

- [Coding standards](/documentation/standards/coding_standards/README.md) — the language and
  size rules these gates enforce.
