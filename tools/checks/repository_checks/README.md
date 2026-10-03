# Repository checks

The `repository_checks` crate: the repository's structural, language-ban, licensing and registry
checks — the engine-layer, workspace-law, `@route` tag and ORBAT coherency gates, the shell,
Python and Node bans with the file-length gate, the upstream code-leak gate and the object registry
alias gate — and the tests that hold every tool crate, found by folder, to the tooling dependency,
structure and prose rules. The `cargo xtask verify` verbs print their reports and exit with their
codes.

## Contents

```text
tools/checks/repository_checks/
├── Cargo.toml  the `repository_checks` library package: `verification_core`, `process_runner`, `repository_laws`, `repository_layout`, layout tier 2
└── src/        the four check groups, the tooling tests, the error and the prelude
```

## How it works

Every check reads the checkout, prints its own report and returns its exit status: 0 held, 1 a
finding, 2 an input it could not read. The engine-layer and workspace-law gates print the reports
of [`repository_laws`](/tools/foundation/repository_laws/README.md) line for line, so the gates and
the `api` engineering-law tests judge the tree the same way. The tooling tests find the tool crates
by folder — every `tools/<name>/Cargo.toml` and every `tools/<category>/<name>/Cargo.toml` — so a
crate is held to the rules from the commit that creates it. The
[source README](/tools/checks/repository_checks/src/README.md) lists each group and the tests.

## Getting started

Run these from the repository root:

```bash
cargo test -p repository_checks   # every check over fixtures and this checkout, and the tooling rules
cargo xtask verify engine-layers  # the engine-layer gate over this checkout
cargo xtask verify no-shell       # the tracked language ban
```

## Configuration

No feature and no environment variable.

## Public surface

- The modules `architecture`, `language_bans`, `licensing` and `registry`; `Error` and `Result` at
  the crate root; `prelude` with each check's entry point.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `repository_laws`, `repository_layout`,
  `regex`, `serde_json`, `syn` and `thiserror`; `tool_test_support`, `toml` and `walkdir` in tests.
- Used by: `xtask` (`cargo xtask verify`, the `ci` task table, and the wave-gate linkage pins of
  its CI schema-parity and faction-library checks).
- Rules: tier 2 of `tools/checks` (`cargo xtask verify crate-tiers`); the crate anatomy
  (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Check crates](/tools/checks/README.md) — the category this crate belongs to.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the size, placement
  and language rules these checks hold.
- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the layer walls.
