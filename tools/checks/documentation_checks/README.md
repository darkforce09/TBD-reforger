# Documentation checks

The `documentation_checks` crate: the documentation link check, `link-check` (every link reaches
what it names, and every path and `cargo xtask` command a live document writes as code exists).
`cargo xtask verify link-check` prints its report and exits with its code.

## Contents

```text
tools/checks/documentation_checks/
├── Cargo.toml  the `documentation_checks` library package: `verification_core`, `process_runner`, `repository_layout`, `regex`, `clap`, layout tier 2
└── src/        the link check, the request and run it uses, the tracked tree, the scope and the regions
```

## How it works

The link check lists the checkout once through `git ls-files` (joined by the untracked files git does
not ignore under `--with-untracked`), resolves the `--path` scope, judges every item in it, and
prints one `verification_core` verdict per item and its totals: exit 0 when every item held, 1 when
one broke a rule, 2 when a check did not run. The link check judges `cargo xtask` citations against
the `CommandVocabulary` its caller hands in — xtask's clap command tree, the build recipe names and
the CI task names — so the crate never reads xtask's command line itself. The
[source README](/tools/checks/documentation_checks/src/README.md) states every rule, the Contents
grammar and the exemptions; the [README standard](/documentation/standards/readme_standard.md) is
the rule set the gates enforce.

## Getting started

Run these from the repository root:

```bash
cargo test -p documentation_checks          # the link check over fixture checkouts
cargo xtask verify link-check --path tools  # the link check over one folder
```

## Configuration

No feature and no environment variable.

## Public surface

- The module `link_check`; at the crate root `verify_link_check`, `GateRequest`,
  `UntrackedFiles`, `BreakListing` and `CommandVocabulary`; `link_check` also exposes the citation
  walk (`xtask_command_tree`, `citations`, `command_words`, `walk_command`, `CitedCommand`) for the
  binary's tests over its own tree; `prelude` with each gate's entry point.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `repository_layout`, `regex` and `clap`.
- Used by: `xtask` (`cargo xtask verify link-check`).
- Rules: tier 2 of `tools/checks` (`cargo xtask verify crate-tiers`); the crate anatomy
  (`cargo xtask verify crate-anatomy`); no command module of xtask is imported.

## Related documentation

- [Check crates](/tools/checks/README.md) — the category this crate belongs to.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  documentation tree's layout, placement and size rules.
