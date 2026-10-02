# Repository laws

The structural engineering laws of the repository as pure checks over a checkout: file length,
test placement, the absence of any exemption mechanism, the engine layer walls, the dependency
direction between the website crates, and the workspace laws over the members of the root
manifest. `cargo xtask verify file-length`, `cargo xtask verify engine-layers` and the five
workspace-law verbs (`cargo xtask verify crate-tiers` and its siblings) print these results, and
the `engineering_laws` test binary of `website-api` asserts on them, so the gates and that binary
never disagree about the tree.

## Contents

```text
tools_v2/verification-core/src/repository_laws/
├── cargo_manifest/            the manifest reader's lexical helpers
├── cargo_manifest.rs          a `Cargo.toml` reader: package keys, dependency edges in every table, features, layout, lints, targets, workspace
├── crate_dependencies.rs      the dependency-direction rules of the website crates and the test-only feature rule
├── engine_layers/             the eight engine-layer walls and the map engine's UI-framework ban
├── exemption_mechanisms.rs    exemption files, comment directives and exemption tables
├── file_length.rs             the 500 and 1000 line ceilings and their report lines
├── mod.rs                     the module tree and the laws' shared contract
├── sibling_test_placement.rs  inline test-module bodies in production files, found by name or by `cfg`
├── source_roots.rs            the roots every structural law walks and the test-file rule
├── tests/                     unit tests for each module and the throwaway checkout they plant files in
├── workspace_laws/            crate tiers, crate anatomy, strangler, frontend layering and Tailwind sources
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
| Engine layers | `engine_layers::check_engine_layers` | the graphics engine, the map engine and the frontend | a breach of rule 1, 2, 3a, 3b, 4, 5, 6 or 7 |
| UI-framework ban | `engine_layers::map_engine_ui_framework_findings` | the map engine's manifest and sources | a UI framework dependency edge or import anywhere in the crate |
| Crate directions | `crate_dependencies::crate_dependency_findings` | the four website crate manifests | an edge against the layer order, in any dependency table |
| Test-only feature | `crate_dependencies::test_only_feature_findings` | one parsed manifest | a feature that a non-test build could carry |
| Workspace laws | `workspace_laws::{crate_tiers, crate_anatomy, strangler, frontend_layering, tailwind_sources}` | the root manifest's members, their manifests and sources, the app stylesheet | see the [workspace laws README](/tools_v2/verification-core/src/repository_laws/workspace_laws/README.md) |

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
manifests use, so the crate keeps its two dependencies. It reads both spellings of a workspace
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
- `workspace_laws`: see its [README](/tools_v2/verification-core/src/repository_laws/workspace_laws/README.md).
- `crate_dependencies`: the four rules and `CRATE_DEPENDENCY_RULES`, `rule_findings`,
  `crate_dependency_findings`, `test_only_feature_findings`, `DependencyFinding`.
- `engine_layers`: see its [README](/tools_v2/verification-core/src/repository_laws/engine_layers/README.md).

## Boundaries

- Depends on: `crate::scan`, `crate::pattern` and `crate::verdict`; `std` only otherwise.
- Used by: `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/` (`verify
  file-length`), `tools_v2/xtask/src/verifications/architecture/engine_layer_boundaries.rs`
  (`verify engine-layers`), `tools_v2/xtask/src/verifications/architecture/workspace_laws.rs`
  (the five workspace-law verbs), and `apps/website/api_v2/tests/engineering_laws.rs`.
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
    (`a_forbidden_edge_in_any_table_or_spelling_is_a_finding`).

## Related documentation

- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the size and test
  placement rules these laws hold.
- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the layer walls.
