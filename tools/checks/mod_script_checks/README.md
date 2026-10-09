# Mod script checks

The `mod_script_checks` crate: the checks over the [EnfScript](/documentation/glossary/a_to_f.md#enfscript)
sources and `.layout` files of the [mod](/documentation/glossary/g_to_m.md#mod) — the UI layout gate and the two
[Workbench](/documentation/glossary/n_to_z.md#workbench) spawn runs. The `cargo xtask verify` and
`cargo xtask mod` verbs print their reports and exit with their codes.

## Contents

```text
tools/checks/mod_script_checks/
├── Cargo.toml  the `mod_script_checks` library package: `verification_core`, `process_runner`, `content_digest`, `repository_layout`, layout tier 2
└── src/        every check, the comment-card rules, the script lexer, the layout parser, the error and the prelude
```

## How it works

Every check takes the checkout root, reads committed files under `mod/`, prints its own
report and returns its exit status: 0 held, 1 a violation, 2 a check that did not run. The source
pins prove each ban and pin on a perturbed copy that must fail, so a check that can no longer fail
is itself a failure. The spawn runs drive a running Workbench through `cargo xtask mcp`
subprocesses; a live run that cannot go on prints its `FATAL:` line and returns 2. The
[source README](/tools/checks/mod_script_checks/src/README.md) lists each check, what it reads and
what it holds.

## Getting started

Run these from the repository root:

```bash
cargo test -p mod_script_checks           # every check over fixtures and this checkout
cargo xtask verify ui-layouts             # the layout structure gate
cargo xtask mod spawn-determinism --selftest  # the spawn-determinism normaliser, offline
```

## Configuration

No feature. The spawn runs read `ENFUSION_WORKBENCH_PORT` (5775 by default), `TBD_DET_TIMEOUT`
(seconds, 120 by default) and `TBD_DET_KEEP` (`1` keeps the run snapshots).

## Public surface

- The modules `ui_layouts`, `spawn_determinism` and `spawn_verification`; `Error` and `Result`
  at the crate root; `prelude` with the `verify ui-layouts` entry.

## Boundaries

- Depends on: `verification_core`, `process_runner`, `content_digest`, `repository_layout`,
  `regex` and `thiserror`.
- Used by: `xtask` (`cargo xtask verify ui-layouts`, `cargo xtask mod spawn-determinism`
  and `spawn-verify`, and the `ci` task table).
- Rules: tier 2 of `tools/checks` (`cargo xtask verify crate-tiers`); the crate anatomy
  (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Check crates](/tools/checks/README.md) — the category this crate belongs to.
- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) — running the Workbench spawn
  checks.
