**Status:** live

# File size and complexity

Rules SIZE-1, SIZE-2, SIZE-3 and COMP-1: how large a source file and a function may grow. SIZE-3
is the live, gated rule; CLAUDE.md law 7 states the same ceilings. The gate's walk and exit codes
are in the [file length and Node ban README](/tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/README.md).

## Rules

- **SIZE-3 (Scalability) — A production Rust file holds at most 500 lines and a test Rust file at
  most 1000, with no exemption.** Lines are raw lines, comments and blank lines included. A file
  is a test file when a component of its path is `tests` or its name ends in `_tests.rs`. A file
  over its limit prints one `SIZE-3:` line and fails the gate; the fix is to decompose it by
  responsibility. Gate: CI-SCRIPT, `cargo xtask verify file-length`, run by
  `cargo xtask ci verify-coding-standards` in `ci-local` and by the `language-gates` job of
  `.github/workflows/ci.yml`. The limits are `SIZE_3_PRODUCTION_MAX_LINES` and
  `SIZE_3_TEST_MAX_LINES` in
  [node_and_file_limits.rs](/tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs).
- **SIZE-2 (Scalability) — File-level exemptions.** Retired. No allowlist file exists, none may be
  created, and the gate reads none: a planted exemption file does not exempt anything
  (`size3_has_zero_exemptions_even_if_allowlist_is_attempted` and `allowlist_file_must_not_exist`
  in `tools_v2/xtask/src/tests/node_free_tests.rs`).
- **SIZE-1 (Scalability) — A soft warning at 600 lines.** Retired; SIZE-3's hard limit replaces
  it. Some code comments still describe a large file as "a SIZE-1 file"; read that as a file near
  the SIZE-3 limit.
- **COMP-1 (Readability) — A function has at most 15 independent paths.** A function past that is
  split into named helpers; the only escape is a per-function opt-out with the reason beside it.
  Status: live, unenforced: it was gated by the Go and TypeScript linters, and no Rust complexity
  lint is configured (no `clippy.toml`, no `[lints]` table); a few functions carry
  `#[allow(clippy::too_many_lines)]` for a lint that is not switched on.

## What the walk covers

`cargo xtask verify file-length` walks every `.rs` file under these roots, untracked files
included, since it reads the working tree:

| Root | Holds |
|---|---|
| `tools_v2/xtask`, `tools_v2/verification-core`, `tools_v2/ticket-engine`, `tools_v2/developer-tools` | the tooling crates, their tests included |
| `apps/ticketboard/src` | the ticketboard |
| `apps/fleet_host_agent/src`, `apps/fleet_host_agent/tests` | the fleet host agent |
| `apps/website/api_v2/src`, `apps/website/frontend/src` | the API and the app |
| every `src/` and `tests/` folder directly under `apps/website/` | the engines, and any crate added there |

A pinned root that is missing, an unreadable file or a walk that finds no `.rs` file is a check
that did not run (exit 2 or 1), never a pass. Generated Rust is not excluded: the contract types
under `apps/website/api_v2/src/missions/contract/generated/` are held to the same limit.

Outside the walk, and so unenforced by this gate:

- [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) `.c` files under `apps/mod/`. CLAUDE.md
  law 7 applies to them, but no gate measures them, and 52 tracked scripts of the three TBD
  addons run past 500 lines.
- Markdown. Live documents under `documentation_v2/` have their own 500-line limit, checked by
  `cargo xtask verify markdown-placement`.

The app's `src/v2/` tree has a second 500-line check in its own tests,
`v2_production_files_meet_the_documentation_standard` in
`apps/website/frontend/src/v2/tests/doc_audit/mod.rs`. That audit keeps a dated exemption table
in `apps/website/frontend/src/v2/tests/doc_audit/allowlist.rs`; the table is empty, and SIZE-3
exempts nothing whatever it holds.
