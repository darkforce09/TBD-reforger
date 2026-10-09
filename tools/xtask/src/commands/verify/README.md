# Verify command group

The `cargo xtask verify` group: the command line of every repository verification. Each verb
calls one check of a check or command crate under `tools/` and exits with that check's code, so a
developer or a CI job runs any single check by name.

## Contents

```text
tools/xtask/src/commands/verify/
├── cli.rs       the `VerifyCmd` clap enum: sixteen verbs, and the flags of the link check
├── dispatch.rs  finds the checkout root and calls the verification behind each verb
└── mod.rs       the module tree
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `VerifyCmd` as the `verify` group. `dispatch::run` matches
the verb, resolves the checkout root with `repository_layout::prelude::find_repository_root` where the
check takes one, and returns the check's exit code unchanged. The folder holds no check logic:
`DocumentationGateArgs` (`--path`, `--with-untracked`) becomes the `GateRequest` of the link
check, and `--report` picks every break over the first ones.

The shared exit convention: 0 the check held; 1 it found a violation; 2, for the checks that tell
the cases apart, it could not run (a missing input, an unreadable file, an empty scan). An error
that escapes a check prints `xtask: <cause>` and exits 1, and a clap usage error exits 2. The
owning folder's README gives each check's codes and rules.

## Commands

Run each as `cargo xtask verify <verb>` from the repository root.

### Language and size gates

- Synopsis: `verify no-python`; `verify no-shell`; `verify no-node`; `verify file-length`
- Does: the hard-zero ban on tracked shell, Make, Python and Node scripts (`no-python` and
  `no-shell` run one walk); Node only as the enfusion-mcp runtime; a warning for every
  production file over 500 lines, over the Rust source trees and the pinned mod Scripts roots
  (`mod/tbd-framework/Scripts`, `mod/tbd-emcp/Scripts`), which never fails the gate. Body:
  `tools/checks/repository_checks/src/language_bans/`.
- Example: `cargo xtask verify no-shell`

### Workspace laws

- Synopsis: `verify crate-tiers`; `verify crate-anatomy`; `verify test-file-reachability`;
  `verify frontend-layering`; `verify tailwind-sources`
- Does: the five workspace laws of the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md) over the members of
  the root manifest: no edge onto an application package in any table and the external-crate
  firewalls; the library crate anatomy; every file in a member's test folders compiled by one of
  its targets; the frontend layer and crate orders, the two shell crates peers (hard at zero); an
  `@source` line per leptos crate. Body:
  `tools/checks/repository_checks/src/architecture/workspace_laws.rs`.
- Example: `cargo xtask verify crate-tiers`

### Database gate

- Synopsis: `verify no-select-star`
- Does: no bare `SELECT *` or `RETURNING *` on tables with nullable columns. Body:
  `tools/commands/database_operations/src/database_checks/`.
- Example: `cargo xtask verify no-select-star`

### Mod script gate

- Synopsis: `verify ui-layouts`
- Does: the structure of the mod's `.layout` files. Body: `tools/checks/mod_script_checks/src/`.
- Example: `cargo xtask verify ui-layouts`

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

### Link check

- Synopsis: `verify link-check [--report] [--path <DIR>]... [--with-untracked]`
- Does: every link, backticked path and cited `cargo xtask` command resolves. `--path` limits the
  check to folders, and `--with-untracked` also judges untracked files git does not ignore. Body:
  `tools/checks/documentation_checks/src/`.
- Example: `cargo xtask verify link-check --path tools/xtask`

## Boundaries

- Depends on: the check and command crates named above.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, which mounts the group;
  - the `ci` task table (`tools/commands/ci_task_catalog/src/task_definitions.rs`), whose `verify-*`
    rows spell these verbs and call the same checks in process;
  - the `language-gates` job of `.github/workflows/ci.yml`;
  - people and agents, for any single check.
- Rules: `dispatch.rs` only routes and never holds check logic; a verb's name is stable once CI or
  a runbook cites it; `cargo xtask verify link-check` fails a live document that cites a verb this
  enum does not declare.

## Related documentation

- [Check crates](/tools/checks/README.md) — the check crates the verbs call.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the rules several
  of these gates enforce.
