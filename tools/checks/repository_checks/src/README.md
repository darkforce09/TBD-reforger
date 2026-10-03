# Repository checks sources

The four check groups of the repository and the tooling tests: the structural gates
(`architecture/`), the language bans and the file-length gate (`language_bans/`), the upstream
code-leak gate (`licensing/`) and the object registry alias gate (`registry/`). Each check is a
`cargo xtask verify` verb.

## Contents

```text
tools/checks/repository_checks/src/
├── architecture/   engine layers, the five workspace laws, route tags, ORBAT coherency and the wave-gate linkage pins
├── error.rs        `Error` and `Result`: a check that could not find the checkout, read a file or start a program
├── language_bans/  the shell, Python and Node bans and the file-length gate
├── lib.rs          the crate root: the checks' shared contract, `mod` lines and the re-exports
├── licensing/      the upstream code-leak gate over the licensed reference lanes
├── prelude.rs      each check's entry point for glob import
├── registry/       the object registry alias gate
└── tests/          the tooling rules over every tool crate: dependency direction, structure and prose
```

## How it works

| Group | Verbs | README |
|---|---|---|
| `architecture` | `engine-layers`, `route-tags`, `editor-orbat-coherency`, `crate-tiers`, `crate-anatomy`, `strangler`, `frontend-layering`, `tailwind-sources` | [architecture](/tools/checks/repository_checks/src/architecture/README.md) |
| `language_bans` | `no-shell`, `no-python`, `no-node`, `file-length` | [language bans](/tools/checks/repository_checks/src/language_bans/README.md) |
| `licensing` | `no-crf-leak` | [licensing](/tools/checks/repository_checks/src/licensing/README.md) |
| `registry` | `object-registry-aliases` | [registry](/tools/checks/repository_checks/src/registry/README.md) |

The tooling tests in `tests/` find the tool crates by folder: every `tools/<name>` that holds a
`Cargo.toml` (the xtask and developer_tools binaries) and every `tools/<category>/<name>` that holds
one. `tooling_dependency_boundaries.rs` holds the dependency direction of the binaries, the
foundation and ticket crate edges, the line limits (production files under 500 lines, test files
under 1000, `tools/xtask/src/main.rs` under 150, the developer_tools binaries under 250) and the
sibling test placement over every one of them. `tooling_prose_rules.rs` walks `git ls-files tools`
for ticket identifiers, retired spellings, script file names, private network addresses, history
narration and Rust file names that exist nowhere; a production source spells a repository path
only in a layout module — a file of the `repository_layout` crate, or a file whose first line
declares it, `//! The repository locations only …`.

## Public surface

- `architecture`, `language_bans`, `licensing`, `registry`: each group's gates (see the group
  READMEs); `Error`, `Result` and `prelude` at the crate root.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `repository_laws`, `repository_layout`,
  `regex`, `serde_json`, `syn`, `thiserror`.
- Used by: `tools/xtask/src/commands/verify/dispatch.rs`, the `ci` task table in
  `tools/commands/ci_task_catalog/src/task_definitions.rs`, and the wave-gate linkage pins of
  `tools/commands/ci_task_catalog/src/workflow_checks/` and `tools/commands/database_operations/src/database_checks/`.
- Rules:
  - a check that could not read its input never reads as a pass (each group's README names its
    tests);
  - every tool crate is held to the tooling rules from the commit that creates it
    (`tooling_crate_folders_are_found_by_folder`).

## Related documentation

- [Coding standards](/documentation/standards/coding_standards/README.md) — the rules these
  checks hold.
