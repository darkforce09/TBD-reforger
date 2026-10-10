**Status:** live

# File size and complexity

Rules SIZE-1, SIZE-2, SIZE-3 and COMP-1: how large a source file and a function may grow. SIZE-3
is strong guidance, not a CI hard gate; CLAUDE.md law 7 states the same limits. The gate's walk and exit codes
are in the [file length and Node ban README](/tools/checks/repository_checks/src/language_bans/node_and_file_limits/README.md).

## Rules

- **SIZE-3 (Scalability) — A production source file stays around 500 lines and a test file
  around 1000.** Lines are raw lines, comments and blank lines included. A file is a test file
  when a component of its path is `tests` or its name ends in `_tests.rs` or `_tests.c`. When a
  file grows past its limit, split it by responsibility. `cargo xtask verify file-length` reports
  long files as warnings; it is not a CI hard gate. The limits are `SIZE_3_PRODUCTION_MAX_LINES` and
  `SIZE_3_TEST_MAX_LINES` in
  [node_and_file_limits.rs](/tools/checks/repository_checks/src/language_bans/node_and_file_limits.rs).
- **SIZE-2 (Scalability) — File-level exemptions.** Retired: with SIZE-3 a warning, no exemption
  file is needed.
- **SIZE-1 (Scalability) — A soft warning at 600 lines.** Retired; SIZE-3's 500-line warning
  replaces it. Some code comments still describe a large file as "a SIZE-1 file"; read that as a file near
  the SIZE-3 limit.
- **COMP-1 (Readability) — A function has at most 15 independent paths.** A function past that is
  split into named helpers; the only escape is a per-function opt-out with the reason beside it.
  Status: live, unenforced: it was gated by the Go and TypeScript linters, and no Rust complexity
  lint is configured (the root `clippy.toml` sets only `allow-unwrap-in-tests`, and the
  `[workspace.lints]` policy of the root `Cargo.toml` switches on no complexity lint); a few
  functions carry `#[allow(clippy::too_many_lines)]` for a lint that is not switched on.

## What the walk covers

`cargo xtask verify file-length` walks every `.rs` and
[EnfScript](/documentation/glossary/a_to_f.md#enfscript) `.c` file under these roots, untracked
files included, since it reads the working tree, and prints
`scanned N source file(s) (R .rs, C .c)`:

| Root | Holds |
|---|---|
| the folder of every workspace member the root `Cargo.toml` names | every crate, its tests, benches and build script included; a crate is walked from the commit that makes it a member |
| `mod/tbd-framework/Scripts` | the shipping game mod's EnfScript |
| `mod/tbd-emcp/Scripts` | the Enfusion MCP bridge's Workbench handlers |

Generated Rust is not excluded: the contract types under
`crates/contracts/contract_schema_types/src/generated/` are reported like any other file.

Outside the walk:

- The addon scripts of `mod/tbd-export`, until they are pinned.
  `MOD_SCRIPT_ROOTS` in
  [node_and_file_limits.rs](/tools/checks/repository_checks/src/language_bans/node_and_file_limits.rs)
  names the three roots the gate may pin, one addon at a time once its scripts meet the
  ceilings: `mod/tbd-framework/Scripts` and `mod/tbd-emcp/Scripts` are pinned, and
  `mod/tbd-export/Scripts` follows (ticket `modularise-document-gate-mod` in `ttm`). The gitignored
  reference lanes in `mod/References/` are never pinned; a compile-time assertion rejects
  any `mod` pin outside the three roots.
- Rust files outside every member folder: none exist. The URL case table the API and the
  single-page app share is `crates/foundation/http_url_guard/src/cases.rs`, a module of a member
  crate, so the walk covers it.
- Markdown. Live documents under `documentation/` follow their own 500-line guidance.
