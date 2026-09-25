# File length and Node ban checks

The bodies of two repository gates, `cargo xtask verify file-length` and
`cargo xtask verify no-node`, plus the Spleen font table generator that `cargo xtask gen font-table`
runs. The parent file `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs`
holds their constants and re-exports the entry points.

## Contents

```text
tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/
├── repository_access.rs  the file-length walk and verdict, the test-file rule, and the font table generator
└── verify_no_node.rs     the no-node gate: tracked Node scripts, node and npx calls, setup-node steps
```

## How it works

`verify file-length` walks every `.rs` file under the roots in `FILE_LENGTH_PINS` (the four
`tools_v2` crates, `apps/ticketboard/src`, `apps/fleet_host_agent/src` and `tests`,
`apps/website/api_v2/src`, `apps/website/frontend/src`) plus every `src/` and `tests/` folder
directly under `apps/website/`. A file is a test file when a path component is `tests` or its stem
ends in `_tests`; a test file may hold 1000 lines (`SIZE_3_TEST_MAX_LINES`), any other file 500
(`SIZE_3_PRODUCTION_MAX_LINES`). There is no exemption list. Each file over its limit prints one
`SIZE-3:` line. A missing root or an unreadable file is a check that did not run, never a pass, and
so is a walk that found no `.rs` file at all.

`verify no-node` runs three checks and counts each failure:

1. `git ls-files '*.mjs' '*.cjs'` outside `apps/mod/` lists nothing.
2. No line of a `.sh`, `.yml` or `.yaml` file under `SCAN_DIRS` (`.github`) or a file in
   `SCAN_FILES` (empty) calls `node ` or `npx ` in command position; `#` and `//` comment lines are
   skipped. A declared subject that is missing or unreadable is a failure, so deleting a file
   cannot quietly shrink the scan.
3. No workflow uses `actions/setup-node`.

`gen_font_table` reads a Spleen 16x32 BDF file, requires a full 16x32 cell for every character from
U+0020 to U+007E, previews four glyphs on stderr and prints the generated Rust table on stdout.

Exit codes: `file-length` 0 clean, 1 at least one file over its limit or an empty walk, 2 did not
run; `no-node` 0 clean, 1 any check failed.

## Boundaries

- Depends on: the constants and the `verification_core` imports of the parent file;
  `verification_core::scan::walk_files` and its `NotRun` causes; `git` for the tracked-file list.
- Used by: `tools_v2/xtask/src/commands/verify/dispatch.rs` (`verify file-length`,
  `verify no-node`); `tools_v2/xtask/src/commands/generate/dispatch.rs` (`gen font-table`); the
  `verify-coding-standards` and `verify-no-node` rows of
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`; the `language-gates` job of
  `.github/workflows/ci.yml`; the platform [wave](/documentation_v2/glossary.md#wave) gate, which runs
  `verify no-node`.
- Rules: a walk that reads nothing is never a pass (`walk_is_nonempty_anti_vacuity`,
  `missing_walk_root_is_did_not_run` and `empty_walk_is_not_ok` in
  `tools_v2/xtask/src/tests/node_free_tests.rs`); the limits are exactly 500 and 1000 lines with no
  exemption (`production_boundary_is_500_lines`,
  `size3_has_zero_exemptions_even_if_allowlist_is_attempted`).

## Related documentation

- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the file size and
  language rules these gates hold.
