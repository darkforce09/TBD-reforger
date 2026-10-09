# Verify command group

The `cargo xtask verify` group: the command line of every repository verification. Each verb
calls one check of a check or command crate under `tools/` and exits with that check's code, so a
developer, a CI job or a [wave](/documentation/glossary/n_to_z.md#wave) gate runs any single check
by name.

## Contents

```text
tools/xtask/src/commands/verify/
├── cli.rs       the `VerifyCmd` clap enum: twenty-six verbs, and the flags the documentation gates share
├── dispatch.rs  finds the checkout root and calls the verification behind each verb
└── mod.rs       the module tree
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `VerifyCmd` as the `verify` group. `dispatch::run` matches
the verb, resolves the checkout root with `repository_layout::prelude::find_repository_root` where the
check takes one, and returns the check's exit code unchanged. The folder holds no check logic:
`DocumentationGateArgs` (`--path`, `--with-untracked`) becomes the `GateRequest` of the
documentation gates, and `--report` of `link-check` picks every break over the first ones.

The shared exit convention: 0 the check held; 1 it found a violation; 2, for the checks that tell
the cases apart, it could not run (a missing input, an unreadable file, an empty scan). An error
that escapes a check prints `xtask: <cause>` and exits 1, and a clap usage error exits 2. The
owning folder's README gives each check's codes and rules.

## Commands

Run each as `cargo xtask verify <verb>` from the repository root.

### Language and size gates

- Synopsis: `verify no-python`; `verify no-shell`; `verify no-node`; `verify file-length`
- Does: the hard-zero ban on tracked shell, Make, Python and Node scripts (`no-python` and
  `no-shell` run one walk); Node only as the enfusion-mcp runtime; 500 lines per production file
  and 1000 per test file, over the Rust source trees and the pinned mod Scripts roots
  (`apps/mod/tbd-framework/Scripts`, `apps/mod/tbd-emcp/Scripts`). Body:
  `tools/checks/repository_checks/src/language_bans/`.
- Example: `cargo xtask verify no-shell`

### Architecture gates

- Synopsis: `verify route-tags`; `verify editor-orbat-coherency`
- Does: every `@route` tag matches
  a registered Axum route and back; the [ORBAT](/documentation/glossary/n_to_z.md#orbat) and Eden
  lock coherency of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator).
  Body: `tools/checks/repository_checks/src/architecture/`.
- Example: `cargo xtask verify route-tags`

### Workspace laws

- Synopsis: `verify crate-tiers`; `verify crate-anatomy`; `verify test-file-reachability`;
  `verify frontend-layering`; `verify tailwind-sources`
- Does: the five workspace laws of the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md) over the members of
  the root manifest: membership (every member judged, the applications included, or one of the
  two tool binaries), layout, tiers, category edges, firewalls and no edge onto an application
  package in any table; the library crate anatomy; every file in a member's test folders compiled
  by one of its targets; the frontend layer and crate orders, the two shell crates peers (hard at
  zero); an `@source` line per leptos crate. Body: `tools/checks/repository_checks/src/architecture/workspace_laws.rs`.
- Example: `cargo xtask verify crate-tiers`

### CI gates

- Synopsis: `verify ci-shell`; `verify ci-schema-parity`
- Does: every GitHub Actions `run:` is a `cargo xtask` call or a short allowed setup line; the
  schema gate set of `ci-local`, the `ci.yml` schema job and the wave gates agree. Body:
  `tools/commands/ci_task_catalog/src/workflow_checks/`.
- Example: `cargo xtask verify ci-shell`

### Database gates

- Synopsis: `verify no-select-star`; `verify wiki-seeds`; `verify faction-library-seeds`
- Does: no bare `SELECT *` or `RETURNING *` on tables with nullable columns; `cargo xtask db seed`
  applies the wiki seed; the faction library seed reaches the database. Body:
  `tools/commands/database_operations/src/database_checks/`.
- Example: `cargo xtask verify no-select-star`

### Deployment gate

- Synopsis: `verify staging-compose-paths`
- Does: every compose command of `cargo xtask deploy website` names
  `deploy/compose.staging.yml`, and `cargo xtask deploy staging` runs none. Body:
  `tools/commands/deployment/src/deployment_checks/`.
- Example: `cargo xtask verify staging-compose-paths`

### Mod script gates

- Synopsis: `verify mission-rest-size-limits`; `verify player-identity-comments`;
  `verify results-reporter-identity-comments`; `verify destroy-target-diagnostics`;
  `verify enfusion-comments [--path <PATH>]...`; `verify ui-layouts`
- Does: the 8 MiB [mission](/documentation/glossary/g_to_m.md#mission) ceiling is checked before the
  [mod](/documentation/glossary/g_to_m.md#mod) parses a document; three comment contracts in the mod
  sources; the in-code documentation card (rules ECM-1 to ECM-9) over the scripts under the pinned
  roots (`apps/mod/tbd-framework/Scripts` and `apps/mod/tbd-emcp/Scripts`;
  `cargo xtask ci verify-coding-standards` and the CI
  `language-gates` job run it that way), or under `--path` (a file or folder in `apps/mod`),
  exiting 2 when a root is missing or the walk is empty; the structure of the mod's `.layout` files. Body:
  `tools/checks/mod_script_checks/src/`.
- Example: `cargo xtask verify mission-rest-size-limits`

### Registry and licensing gates

- Synopsis: `verify object-registry-aliases`; `verify no-crf-leak`
- Does: every Objects-palette alias has its row in the mod's spawn registry; no identifier or
  asset GUID of the upstream reference frameworks reaches the mod. Bodies:
  `tools/checks/repository_checks/src/registry/` and `tools/checks/repository_checks/src/licensing/`.
- Example: `cargo xtask verify object-registry-aliases`

### Map asset gate

- Synopsis: `verify blas-manifest`
- Does: the prefab BLAS library is complete and matches its manifest. Body:
  `verify_blas_manifest` in `tools/map_assets/map_asset_verification/src/blas_manifest.rs`.
- Example: `cargo xtask verify blas-manifest`

### api-readiness

- Synopsis: `verify api-readiness [--evidence <DIR>] [--execute]`; `--evidence` defaults to
  `target/api-readiness`.
- Does: judges every requirement of the [API](/documentation/glossary/a_to_f.md#api) acceptance
  register against the evidence receipts in `DIR`; `--execute` first runs the registered local
  checks. Body:
  `tools/commands/api_readiness_checks/src/`.
- Example: `cargo xtask verify api-readiness`

### Documentation gates

- Synopsis: `verify readme-coverage [--path <DIR>]... [--with-untracked]`;
  `verify markdown-placement [--path <DIR>]... [--with-untracked]`;
  `verify link-check [--report] [--path <DIR>]... [--with-untracked]`
- Does: every folder of the code trees and the documentation root has a README whose Contents
  block matches the folder; Markdown sits only where it belongs and live documents stay within 500
  lines; every link, backticked path and cited `cargo xtask` command resolves. `--path` limits a
  gate to folders, and `--with-untracked` also judges untracked files git does not ignore. Body:
  `tools/checks/documentation_checks/src/`.
- Example: `cargo xtask verify readme-coverage --path tools/xtask`

## Boundaries

- Depends on: `crate::verifications` (every group named above); `tool_test_support`.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, which mounts the group;
  - the `ci` task table (`tools/commands/ci_task_catalog/src/task_definitions.rs`), whose `verify-*`
    rows spell these verbs and call the same checks in process, and whose `ci-local` runs
    `verify ci-schema-parity`;
  - the `language-gates` and `mod-gates-hosted` jobs of `.github/workflows/ci.yml`;
  - the platform wave gate (`VERIFY_STEPS` in
    `tools/commands/platform_execution/src/wave_execution/gate.rs`, and the language gates in
    `tools/commands/platform_execution/src/wave_execution/gate/gate_dispatch.rs`) and the mod wave
    gate, which run verbs as `cargo run -q -p xtask -- verify <verb>` subprocesses;
  - people and agents, for any single check.
- Rules: `dispatch.rs` only routes and never holds check logic; a verb's name is stable once CI, a
  wave gate or a runbook cites it; `cargo xtask verify link-check` fails a live document that
  cites a verb this enum does not declare.

## Related documentation

- [Check crates](/tools/checks/README.md) — the check crates the verbs call.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the rules several
  of these gates enforce.
