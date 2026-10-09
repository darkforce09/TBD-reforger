# Repository checks

The `repository_checks` crate: the repository's structural, language-ban, licensing and registry
checks — the workspace-law gates, the shell, Python and Node bans with the file-length advice,
the upstream code-leak gate and the object registry alias gate. The `cargo xtask verify` verbs
print their reports and exit with their codes.

## Contents

```text
tools/checks/repository_checks/
├── Cargo.toml  the `repository_checks` library package: `verification_core`, `process_runner`, `repository_laws`, `repository_layout`, layout tier 2
└── src/        the four check groups, the error and the prelude
```

## How it works

Every check reads the checkout, prints its own report and returns its exit status: 0 held, 1 a
finding, 2 an input it could not read. The workspace-law gates print the reports
of [`repository_laws`](/tools/foundation/repository_laws/README.md) line for line, so the gates and
the library judge the tree the same way. The
[source README](/tools/checks/repository_checks/src/README.md) lists each group and the tests.

## Getting started

Run these from the repository root:

```bash
cargo test -p repository_checks   # every check over fixtures and this checkout
cargo xtask verify crate-tiers    # the crate-tier law over this checkout
cargo xtask verify no-shell       # the tracked language ban
```

## Configuration

No feature and no environment variable.

## Public surface

- The modules `architecture`, `language_bans`, `licensing` and `registry`; `Error` and `Result` at
  the crate root; `prelude` with each check's entry point.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `repository_laws`, `repository_layout`,
  `regex`, `serde_json` and `thiserror`; `tool_test_support`, `toml` and `walkdir` in tests.
- Used by: `xtask` (`cargo xtask verify` and the `ci` task table).
- Rules: tier 2 of `tools/checks` (`cargo xtask verify crate-tiers`); the crate anatomy
  (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Check crates](/tools/checks/README.md) — the category this crate belongs to.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the size, placement
  and language rules these checks hold.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-level
  boundary laws.
