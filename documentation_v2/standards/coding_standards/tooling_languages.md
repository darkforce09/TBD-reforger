**Status:** live

# Tooling languages

Rules LANG-1, LANG-2 and LANG-3: which language repository tooling is written in. Every rule here
is live and gated. The gate bodies and their exact matching rules are in the
[language ban gates README](/tools_v2/xtask/src/verifications/language_bans/README.md); this page
states the rules and why they exist.

## Rules

- **LANG-1 (Scalability) — New tooling is Rust, in `xtask`.** Anything that reads a file, parses
  JSON, walks the repository, computes a verdict or generates code is a `cargo xtask` subcommand,
  or a binary of `tools_v2/developer-tools`. A tracked shell script (`sh`, `bash`, `zsh`, `ksh`,
  `fish`, `bat`, `ps1`), a Make file (`GNUmakefile`, `makefile`, `Makefile`, `*.mk`), or an
  extensionless file whose shebang names a shell fails the gate. Gate: CI-SCRIPT,
  `cargo xtask verify no-shell`.
- **LANG-2 (Scalability) — No Python.** No tracked `.py` file, no Python shebang, and no `python3`
  in command position in a tracked file (comment lines do not count). It is the same ban table as
  LANG-1; `cargo xtask verify no-python` runs the same walk under its own name so the CI job name
  stays stable. Gate: CI-SCRIPT, `cargo xtask verify no-python`.
- **LANG-3 (Debuggability) — Both are bans, not ratchets.** One table,
  `TRACKED_LANGUAGE_BANS` in
  [shell_scripts.rs](/tools_v2/xtask/src/verifications/language_bans/shell_scripts.rs), covers
  shell, Make, Python and the Node script extensions `.mjs` and `.cjs`. Any tracked match fails.
  There is no inventory, no allowlist and no "may only shrink" count. No path prefix is skipped:
  `apps/mod/` is scanned like everything else, and [EnfScript](/documentation_v2/glossary.md#enfscript)
  sources pass only because `.c` is not in the table. A failed `git ls-files`, an empty listing or
  an unreadable tracked path fails the gate. Gate: CI-SCRIPT, both command names.

Shell is permitted for one thing only: thin process glue that must run before cargo or without it,
such as a container entry point, a `distrobox-host-exec` wrapper or a git hook. Such glue stays
outside the git tree; a tracked copy fails the gate. A script that parses anything is tooling, and
tooling is Rust.

A companion gate, `cargo xtask verify no-node`, holds Node to the
[Enfusion](/documentation_v2/glossary.md#enfusion) MCP runtime: no tracked `.mjs` or `.cjs` file
outside `apps/mod/`, no `node` or `npx` call in a workflow or shell line under `.github/`, and no
`actions/setup-node` step. It has no rule code; the
[file length and Node ban README](/tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/README.md)
describes it.

## Why the bans exist

Every failure the bans prevent has one shape: a tool that reports success over input it never
examined. Shell makes that shape cheap to write and hard to see in review:

- a missing binary, with `|| true` turning exit status 127 into a silent pass, so a gate never
  executes and still reports green;
- a regular expression whose meaning depends on which `grep` implementation runs it;
- a parameter default that truncates at a brace inside a GUID while the validator prints that the
  configuration is valid;
- a script with no reachable success or failure exit, which passes only against a stale build.

A compiler refuses each of these before the tool runs. That is the whole argument for LANG-1, and
the reason LANG-3 makes an unreadable input a failure rather than a pass.

## Where the gates run

`no-shell`, `no-python` and `no-node` run as steps of `cargo xtask ci ci-local`, in the
`language-gates` job of `.github/workflows/ci.yml` on every push and pull request to `main`, and in
the platform [wave](/documentation_v2/glossary.md#wave) gate. The full matrix is in
[Testing and CI](/documentation_v2/runbooks/testing_and_ci.md#gate-matrix).

The tests that hold the rules: `planted_sh_fails`, `makefile_is_banned` and
`leftover_py_file_fails` in `tools_v2/xtask/src/verifications/language_bans/tests/python_scripts/tests.rs`;
`does_not_sweep_in_rust_inner_attributes` and `python3_command_position_ignores_comments` in
`tools_v2/xtask/src/verifications/language_bans/tests/shell_scripts/tests.rs`.
