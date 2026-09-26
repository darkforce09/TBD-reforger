# Verify command group

The `cargo xtask verify` group: the command line of every repository verification. Each verb
calls one check in `tools_v2/xtask/src/verifications/` and exits with that check's code, so a
developer, a CI job or a [wave](/documentation_v2/glossary/n_to_z.md#wave) gate runs any single check
by name.

## Contents

```text
tools_v2/xtask/src/commands/verify/
├── cli.rs       the `VerifyCmd` clap enum: twenty-five verbs, and the flags the documentation gates share
├── dispatch.rs  finds the checkout root and calls the verification behind each verb
└── mod.rs       the module tree
```

## How it works

`tools_v2/xtask/src/cli/mod.rs` mounts `VerifyCmd` as the `verify` group. `dispatch::run` matches
the verb, resolves the checkout root with `crate::core::repository_root::find_repo_root` where the
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
  `no-shell` run one walk); Node only as the enfusion-mcp runtime; 500 lines per production Rust
  file and 1000 per test file. Body: `tools_v2/xtask/src/verifications/language_bans/`.
- Example: `cargo xtask verify no-shell`

### Architecture gates

- Synopsis: `verify engine-layers`; `verify route-tags`; `verify editor-orbat-coherency`
- Does: the layer rules between the graphics engine and the map engine; every `@route` tag matches
  a registered Axum route and back; the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and Eden
  lock coherency of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator).
  Body: `tools_v2/xtask/src/verifications/architecture/`.
- Example: `cargo xtask verify engine-layers`

### CI gates

- Synopsis: `verify ci-shell`; `verify ci-schema-parity`
- Does: every GitHub Actions `run:` is a `cargo xtask` call or a short allowed setup line; the
  schema gate set of `ci-local`, the `ci.yml` schema job and the wave gates agree. Body:
  `tools_v2/xtask/src/verifications/ci/`.
- Example: `cargo xtask verify ci-shell`

### Database gates

- Synopsis: `verify no-select-star`; `verify wiki-seeds`; `verify faction-library-seeds`
- Does: no bare `SELECT *` or `RETURNING *` on tables with nullable columns; `cargo xtask db seed`
  applies the wiki seed; the faction library seed reaches the database. Body:
  `tools_v2/xtask/src/verifications/database/`.
- Example: `cargo xtask verify no-select-star`

### Deployment gate

- Synopsis: `verify staging-compose-paths`
- Does: `cargo xtask deploy staging` resolves the staging compose file by an absolute path. Body:
  `tools_v2/xtask/src/verifications/deployment/`.
- Example: `cargo xtask verify staging-compose-paths`

### Mod script gates

- Synopsis: `verify mission-rest-size-limits`; `verify player-identity-comments`;
  `verify results-reporter-identity-comments`; `verify destroy-target-diagnostics`;
  `verify ui-layouts`
- Does: the 8 MiB [mission](/documentation_v2/glossary/g_to_m.md#mission) ceiling is checked before the
  [mod](/documentation_v2/glossary/g_to_m.md#mod) parses a document; three comment contracts in the mod
  sources; the structure of the mod's `.layout` files. Body:
  `tools_v2/xtask/src/verifications/mod_scripts/`.
- Example: `cargo xtask verify mission-rest-size-limits`

### Registry and licensing gates

- Synopsis: `verify object-registry-aliases`; `verify no-crf-leak`
- Does: every Objects-palette alias has its row in the mod's spawn registry; no identifier or
  asset GUID of the upstream reference frameworks reaches the mod. Bodies:
  `tools_v2/xtask/src/verifications/registry/` and `tools_v2/xtask/src/verifications/licensing/`.
- Example: `cargo xtask verify object-registry-aliases`

### Map asset gate

- Synopsis: `verify blas-manifest`
- Does: the prefab BLAS library is complete and matches its manifest. Body:
  `tools_v2/xtask/src/verifications/map_assets/`, which calls `developer-tools`.
- Example: `cargo xtask verify blas-manifest`

### api-readiness

- Synopsis: `verify api-readiness [--evidence <DIR>] [--execute]`; `--evidence` defaults to
  `target/api-readiness`.
- Does: judges every requirement of the [API](/documentation_v2/glossary/a_to_f.md#api) acceptance
  register against the evidence receipts in `DIR`; `--execute` first runs the registered local
  checks. Body:
  `tools_v2/xtask/src/verifications/api_readiness/`.
- Example: `cargo xtask verify api-readiness`

### Documentation gates

- Synopsis: `verify readme-coverage [--path <DIR>]... [--with-untracked]`;
  `verify markdown-placement [--path <DIR>]... [--with-untracked]`;
  `verify link-check [--report] [--path <DIR>]... [--with-untracked]`
- Does: every folder of the code trees and the documentation root has a README whose Contents
  block matches the folder; Markdown sits only where it belongs and live documents stay within 500
  lines; every link, backticked path and cited `cargo xtask` command resolves. `--path` limits a
  gate to folders, and `--with-untracked` also judges untracked files git does not ignore. Body:
  `tools_v2/xtask/src/verifications/documentation/`.
- Example: `cargo xtask verify readme-coverage --path tools_v2/xtask`

## Boundaries

- Depends on: `crate::verifications` (every group named above); `crate::core::repository_root`.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, which mounts the group;
  - the `ci` task table (`tools_v2/xtask/src/commands/ci/task_definitions.rs`), whose `verify-*`
    rows spell these verbs and call the same checks in process, and whose `ci-local` runs
    `verify ci-schema-parity`;
  - the `language-gates` and `mod-gates-hosted` jobs of `.github/workflows/ci.yml`;
  - the platform wave gate (`VERIFY_STEPS` in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`, and the language gates in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate/gate_dispatch.rs`) and the mod wave
    gate, which run verbs as `cargo run -q -p xtask -- verify <verb>` subprocesses;
  - people and agents, for any single check.
- Rules: `dispatch.rs` only routes and never holds check logic; a verb's name is stable once CI, a
  wave gate or a runbook cites it; `cargo xtask verify link-check` fails a live document that
  cites a verb this enum does not declare.

## Related documentation

- [Repository verifications](/tools_v2/xtask/src/verifications/README.md) — the check groups the
  verbs call.
- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the rules several
  of these gates enforce.
