# Documentation checks

The `documentation_checks` crate: the three documentation gates — `readme-coverage` (every folder
carries a README.md whose Contents block matches the folder), `markdown-placement` (Markdown lives
in the documentation tree, and live documents stay short) and `link-check` (every link reaches what
it names, and every path and `cargo xtask` command a live document writes as code exists). The
`cargo xtask verify` verbs and the `verify-documentation` row of `cargo xtask ci` print their
reports and exit with their codes.

## Contents

```text
tools/checks/documentation_checks/
├── Cargo.toml  the `documentation_checks` library package: `verification_core`, `process_runner`, `repository_layout`, `regex`, `clap`, layout tier 2
└── src/        the three gates, the request and run they share, the tracked tree, the scope and the regions
```

## How it works

Each gate lists the checkout once through `git ls-files` (joined by the untracked files git does
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
cargo test -p documentation_checks          # every gate over fixture checkouts and this checkout
cargo xtask ci verify-documentation         # the three gates over the committed tree, as CI runs them
cargo xtask verify link-check --path tools  # one gate over one folder
```

## Configuration

No feature and no environment variable.

## Public surface

- The modules `readme_coverage`, `markdown_placement` and `link_check`; at the crate root
  `verify_readme_coverage`, `verify_markdown_placement`, `verify_link_check`, `GateRequest`,
  `UntrackedFiles`, `BreakListing` and `CommandVocabulary`; `link_check` also exposes the citation
  walk (`xtask_command_tree`, `citations`, `command_words`, `walk_command`, `CitedCommand`) for the
  binary's tests over its own tree; `prelude` with each gate's entry point.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `repository_layout`, `regex` and `clap`;
  `tool_test_support` and clap's derive in tests.
- Used by: `xtask` (`cargo xtask verify readme-coverage`, `markdown-placement` and `link-check`,
  and the `verify-documentation` row of the `ci` task table, a `ci-local` step).
- Rules: tier 2 of `tools/checks` (`cargo xtask verify crate-tiers`); the crate anatomy
  (`cargo xtask verify crate-anatomy`); no command module of xtask is imported.

## Related documentation

- [Check crates](/tools/checks/README.md) — the category this crate belongs to.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  documentation tree's layout, placement and size rules.
