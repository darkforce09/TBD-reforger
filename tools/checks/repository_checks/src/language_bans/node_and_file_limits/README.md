# File length and Node ban checks

The bodies of two repository gates, `cargo xtask verify file-length` and
`cargo xtask verify no-node`. The parent file `tools/checks/repository_checks/src/language_bans/node_and_file_limits.rs`
holds the no-node scan subjects and re-exports the entry points.

## Contents

```text
tools/checks/repository_checks/src/language_bans/node_and_file_limits/
├── repository_access.rs  the file-length report over the library scan
└── verify_no_node.rs     the no-node gate: tracked Node scripts, node and npx calls, setup-node steps
```

## How it works

`verify file-length` runs `repository_laws::file_length::scan_file_lengths`
and prints its result. The scan walks every `.rs` and `.c` file under the law roots of
[`repository_laws`](/tools/foundation/repository_laws/src/README.md): the folder of
every workspace member the root `Cargo.toml` names, plus the script roots in `PINNED_SCRIPT_ROOTS`
(`mod/tbd-framework/Scripts`, `mod/tbd-emcp/Scripts`). A file is a test file when a path component is `tests` or
its stem ends in `_tests` (`.rs` or `.c`); a test file may hold 1000 lines (`TEST_MAX_LINES`), any
other file 500 (`PRODUCTION_MAX_LINES`). There is no exemption list. Each file over its limit
prints one `SIZE-3:` line on stderr, and the summary line on stdout reads
`scanned N source file(s) (R .rs, C .c)`. A missing root or an unreadable file is a check that
did not run, never a pass — an explicit workspace member whose folder is missing included — and
so is a walk that found no source file at all. The
`engineering_laws` test binary of `api` reads the same scan, so the gate and that binary
judge the tree the same way.

`MOD_SCRIPT_ROOTS` names the three addon script roots (`mod/tbd-framework/Scripts`,
`mod/tbd-emcp/Scripts`, `mod/tbd-export/Scripts`), the only `mod` trees the law may
pin; a compile-time assertion (`mod_pins_are_script_roots`) rejects any other `mod` pin, so the
gitignored `crf_framework` and `vanilla_reference` references never enter the walk. The framework
and tbd-emcp roots are pinned; the tbd-export root joins once that addon's scripts sit under the
ceilings.

`verify no-node` runs three checks and counts each failure:

1. `git ls-files '*.mjs' '*.cjs'` outside `mod/` lists nothing (`refused_node_scripts`: only a
   path inside the mod folder itself is exempt, never a sibling such as `models/`).
2. No line of a `.sh`, `.yml` or `.yaml` file under `SCAN_DIRS` (`.github`) or a file in
   `SCAN_FILES` (empty) calls `node ` or `npx ` in command position; `#` and `//` comment lines are
   skipped. A declared subject that is missing or unreadable is a failure, so deleting a file
   cannot quietly shrink the scan.
3. No workflow uses `actions/setup-node`.

Exit codes: `file-length` 0 clean, 1 at least one file over its limit or an empty walk, 2 did not
run; `no-node` 0 clean, 1 any check failed.

## Boundaries

- Depends on: the constants and imports of the parent file;
  `repository_laws::file_length` for the roots, ceilings and scan, and its
  `NotRun` causes; `git` for the tracked-file list.
- Used by: `tools/xtask/src/commands/verify/dispatch.rs` (`verify file-length`,
  `verify no-node`); the
  `verify-coding-standards` and `verify-no-node` rows of
  `tools/commands/ci_task_catalog/src/task_definitions.rs`; the `language-gates` job of
  `.github/workflows/ci.yml`; the platform [wave](/documentation/glossary/n_to_z.md#wave) gate, which runs
  `verify no-node`.
- Rules: `.c` files meet the same ceilings (`enfusion_script_production_boundary_is_500_lines`,
  `enfusion_script_tests_basename_holds_to_1000_lines`); a walk that reads nothing is never a pass (`walk_is_nonempty_anti_vacuity`,
  `missing_walk_root_is_did_not_run` and `empty_walk_is_not_ok` in
  `tools/checks/repository_checks/src/language_bans/tests/node_free_tests.rs`); the limits are exactly 500 and 1000 lines with no
  exemption (`production_boundary_is_500_lines`,
  `size3_has_zero_exemptions_even_if_allowlist_is_attempted`).

## Related documentation

- [Coding standards](/documentation/standards/coding_standards/README.md) — the file size and
  language rules these gates hold.
