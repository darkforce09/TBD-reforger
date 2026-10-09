# Repository laws

The structural engineering laws of the repository as pure checks over a checkout: file length,
test placement, the absence of any exemption mechanism, the dependency direction between the
website applications, and the workspace laws over the members of the root manifest.
`cargo xtask verify file-length` and the five workspace-law verbs (`cargo xtask verify crate-tiers` and its siblings) print these results, and
the `engineering_laws` test binary of `api` asserts on them, so the gates and that binary
never disagree about the tree.

## Contents

```text
tools/foundation/repository_laws/src/
├── cargo_manifest/            the manifest reader's lexical helpers
├── cargo_manifest.rs          a `Cargo.toml` reader: package keys, dependency edges in every table, features, layout, lints, targets, workspace
├── crate_dependencies.rs      the dependency-direction rules of the website crates and the test-only feature rule
├── error.rs                   `Error` and `Result`: a law whose input is missing or unreadable
├── exemption_mechanisms.rs    exemption files, comment directives and exemption tables
├── file_length.rs             the 500 and 1000 line ceilings and their report lines
├── lib.rs                     the crate root: the laws' shared contract, `mod` lines and the re-exports
├── prelude.rs                 each law's entry point and the member reader for glob import
├── sibling_test_placement.rs  inline test-module bodies in production files, found by name or by `cfg`
├── source_roots.rs            the roots every structural law walks and the test-file rule
├── tests/                     unit tests for each module and the throwaway checkout they plant files in
├── workspace_laws/            crate tiers, crate anatomy, test-file reachability, frontend layering and Tailwind sources
└── workspace_members.rs       the root manifest's members: explicit folders and globs, minus excludes
```

## How it works

Every law is a function over a repository root that returns its findings, or `NotRun` when an
input it needs is missing or unreadable.

| Law | Function | Reads | Finding |
|---|---|---|---|
| File length | `file_length::scan_file_lengths` | every `.rs` and `.c` file under the law roots | a production file over 500 lines, a test file over 1000 |
| Test placement | `sibling_test_placement::scan_inline_test_modules` | every production `.rs` file under the law roots | a `mod <name> {` body named `tests` or `test`, or one a `cfg` enables under test |
| No exemption | `exemption_mechanisms::scan_exemption_mechanisms` | every file under the law roots, and the files at the repository root | an exemption-list file name, a comment directive switching a structural rule off, a declared exemption table |
| Crate directions | `crate_dependencies::crate_dependency_findings` | the frontend, api and offline service worker manifests | an edge against the layer order, in any dependency table |
| Test-only feature | `crate_dependencies::test_only_feature_findings` | one parsed manifest | a feature that a non-test build could carry |
| Workspace laws | `workspace_laws::{crate_tiers, crate_anatomy, test_file_reachability, frontend_layering, tailwind_sources}` | the root manifest's members, their manifests and sources, the app stylesheet | see the [workspace laws README](/tools/foundation/repository_laws/src/workspace_laws/README.md) |

The law roots are the folder of every workspace member the root `Cargo.toml` names (read by
`workspace_members`; a member nested inside another member is walked once, as part of the outer
one) plus `source_roots::PINNED_SCRIPT_ROOTS`, the framework and tbd-emcp addon script roots. A
missing root manifest, a workspace that names no member, an explicit member folder that is missing
and a missing script root are each `NotRun::TargetMissing`, never a smaller walk, so a crate is
judged from the commit that makes it a member. A file is a test file when a path component is
`tests` or its `.rs` or `.c` stem ends in `_tests`; test files are the sibling files, so the
placement law reads only production files.

The placement law is line-level and `cfg`-aware: the attributes of a module are those on its own
line and on the attribute, doc-comment, comment and blank lines directly above it, and a `cfg`
enables tests when it names `test` outside `not(...)`. The exemption patterns name a structural
rule or an exemption table explicitly, so a host allow-list or a rate-limit exemption is not a
finding, and a directive counts only on a comment line.

The dependency laws read manifests with `cargo_manifest`, a reader for the TOML subset Cargo
manifests use, so the crate needs no TOML dependency. It reads both spellings of a workspace
dependency (`name = { workspace = true }`, `name.workspace = true`) and of an inherited package key,
tells a `[target.'cfg(…)'.dependencies]` table apart from a plain one, and never reads a
`[workspace.dependencies]` entry as an edge. A renamed dependency counts under its real
package name, and a `#` comment never produces an edge.

## Public surface

- `file_length`: `scan_file_lengths`, `FileLengthScan`, `FileLengthViolation`, `line_limit`,
  `length_scan_summary`, `PRODUCTION_MAX_LINES`, `TEST_MAX_LINES`.
- `sibling_test_placement`: `scan_inline_test_modules`, `inline_test_modules_in`,
  `attribute_enables_tests`, `InlineTestModule`, `InlineTestModuleScan`.
- `exemption_mechanisms`: `scan_exemption_mechanisms`, `ExemptionScan`, `ExemptionFinding`,
  `ExemptionKind` and the three patterns.
- `source_roots`: `PINNED_SCRIPT_ROOTS`, `MOD_SCRIPT_ROOTS`, `LENGTH_GATED_EXTENSIONS`,
  `law_source_roots`, `outermost_folders`, the three walks, `repository_relative`, `is_test_file`,
  `mod_pins_are_script_roots`.
- `cargo_manifest`: `read_manifest`, `parse_manifest`, `CargoManifest`, `DependencyEdge`,
  `DependencyKind`, `FeatureDeclaration`, `PackageField`, `LintsSource`, `LayoutDeclaration`,
  `BuildTarget`, `WorkspaceDeclaration`.
- `workspace_members`: `read_workspace_members`, `WorkspaceMember`, `wildcard_matches`.
- `workspace_laws`: see its [README](/tools/foundation/repository_laws/src/workspace_laws/README.md).
- `crate_dependencies`: the three rules and `CRATE_DEPENDENCY_RULES`, `rule_findings`,
  `crate_dependency_findings`, `test_only_feature_findings`, `DependencyFinding`.
- At the crate root: `Error` and `Result`; `prelude`: each law's entry point,
  `WorkspaceLawReport`, `WorkspaceMember` and `read_workspace_members`.

## Boundaries

- Depends on: `verification_core` (`scan`, `pattern` and `verdict`), `regex` and `thiserror`.
- Used by: `tools/checks/repository_checks/src/language_bans/node_and_file_limits/` (`verify
  file-length`), `tools/checks/repository_checks/src/architecture/workspace_laws.rs`
  (the five workspace-law verbs), and `apps/api/tests/engineering_laws.rs`.
- Rules:
  - a missing root or unreadable file is `NotRun`, never zero findings
    (`a_missing_pin_is_a_walk_that_did_not_run`, `an_unreadable_file_is_a_scan_that_did_not_run`,
    `a_missing_manifest_is_a_check_that_did_not_run`);
  - the ceilings are exactly 500 and 1000 lines with no exemption
    (`a_production_file_may_hold_exactly_500_lines`,
    `generated_code_and_website_test_trees_have_no_exemption`);
  - `cfg(not(test))` never counts as a test module and a sibling declaration never counts as a body
    (`the_cfg_predicate_decides_and_not_test_never_counts`,
    `a_sibling_file_declaration_is_not_an_inline_body`);
  - every dependency table and a renamed edge count
    (`a_forbidden_edge_in_any_table_or_spelling_is_a_finding`), and every package a direction
    rule forbids is a member of this workspace
    (`every_forbidden_package_is_a_member_of_this_workspace`).

## Related documentation

- [Coding standards](/documentation/standards/coding_standards/README.md) — the size and test
  placement rules these laws hold.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-level
  boundary laws.
