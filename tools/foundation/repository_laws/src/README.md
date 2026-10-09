# Repository laws

The structural laws of the repository as pure checks over a checkout: the size advice for
production files and the workspace laws over the members of the root manifest (the application
boundary, the firewalls, crate anatomy, test-file reachability, frontend layering and Tailwind
sources). `cargo xtask verify file-length` and the five workspace-law verbs
(`cargo xtask verify crate-tiers` and its siblings) print these results.

## Contents

```text
tools/foundation/repository_laws/src/
├── cargo_manifest/            the manifest reader's lexical helpers
├── cargo_manifest.rs          a `Cargo.toml` reader: package keys, dependency edges in every table, features, layout, lints, targets, workspace
├── error.rs                   `Error` and `Result`: a law whose input is missing or unreadable
├── file_length.rs             the 500-line production size advice and its warning lines
├── lib.rs                     the crate root: the laws' shared contract, `mod` lines and the re-exports
├── prelude.rs                 each law's entry point and the member reader for glob import
├── source_roots.rs            the roots the size advice walks and the test-file rule
├── tests/                     unit tests of the manifest and member readers and the throwaway checkout they plant files in
├── workspace_laws/            crate tiers, crate anatomy, test-file reachability, frontend layering and Tailwind sources
└── workspace_members.rs       the root manifest's members: explicit folders and globs, minus excludes
```

## How it works

Every law is a function over a repository root that returns its findings, or `NotRun` when an
input it needs is missing or unreadable.

| Law | Function | Reads | Finding |
|---|---|---|---|
| File length | `file_length::scan_file_lengths` | every production `.rs` and `.c` file under the law roots | a production file over 500 lines, which the gate prints as a warning |
| Workspace laws | `workspace_laws::{crate_tiers, crate_anatomy, test_file_reachability, frontend_layering, tailwind_sources}` | the root manifest's members, their manifests and sources, the app stylesheet | see the [workspace laws README](/tools/foundation/repository_laws/src/workspace_laws/README.md) |

The law roots are the folder of every workspace member the root `Cargo.toml` names (read by
`workspace_members`; a member nested inside another member is walked once, as part of the outer
one) plus `source_roots::PINNED_SCRIPT_ROOTS`, the framework and tbd-emcp addon script roots. A
missing root manifest, a workspace that names no member, an explicit member folder that is missing
and a missing script root are each `NotRun::TargetMissing`, never a smaller walk, so a crate is
judged from the commit that makes it a member. A file is a test file when a path component is
`tests` or its `.rs` or `.c` stem ends in `_tests`; the size advice never counts a test file.

The dependency laws read manifests with `cargo_manifest`, a reader for the TOML subset Cargo
manifests use, so the crate needs no TOML dependency. It reads both spellings of a workspace
dependency (`name = { workspace = true }`, `name.workspace = true`) and of an inherited package key,
tells a `[target.'cfg(…)'.dependencies]` table apart from a plain one, and never reads a
`[workspace.dependencies]` entry as an edge. A renamed dependency counts under its real
package name, and a `#` comment never produces an edge.

## Public surface

- `file_length`: `scan_file_lengths`, `FileLengthScan`, `FileLengthViolation`,
  `PRODUCTION_MAX_LINES`.
- `source_roots`: `PINNED_SCRIPT_ROOTS`, `MOD_SCRIPT_ROOTS`, `LENGTH_GATED_EXTENSIONS`,
  `law_source_roots`, `outermost_folders`, `walk_law_sources`, `walk_length_gated_sources`,
  `repository_relative`, `is_test_file`, `mod_pins_are_script_roots`.
- `cargo_manifest`: `read_manifest`, `parse_manifest`, `CargoManifest`, `DependencyEdge`,
  `DependencyKind`, `FeatureDeclaration`, `PackageField`, `LintsSource`, `LayoutDeclaration`,
  `BuildTarget`, `WorkspaceDeclaration`.
- `workspace_members`: `read_workspace_members`, `WorkspaceMember`, `wildcard_matches`.
- `workspace_laws`: see its [README](/tools/foundation/repository_laws/src/workspace_laws/README.md).
- At the crate root: `Error` and `Result`; `prelude`: each law's entry point,
  `CrateTierConfiguration`, `WorkspaceLawReport`, `WorkspaceMember` and `read_workspace_members`.

## Boundaries

- Depends on: `verification_core` (`scan`, `pattern` and `verdict`), `regex` and `thiserror`.
- Used by: `tools/checks/repository_checks/src/language_bans/node_and_file_limits/` (`verify
  file-length`) and `tools/checks/repository_checks/src/architecture/workspace_laws.rs` (the five
  workspace-law verbs); the workspace member and manifest readers by several tool crates.
- Rules:
  - a missing root or unreadable file is `NotRun`, never zero findings; the dependency direction
    of every member is the crate-tier law of the
    [workspace laws README](/tools/foundation/repository_laws/src/workspace_laws/README.md).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-level
  boundary laws.
