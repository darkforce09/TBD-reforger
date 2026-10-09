# Architecture verifications

Source checks for the workspace laws over the members of the root manifest. Each law is its own `cargo xtask verify` verb.

## Contents

```text
tools/checks/repository_checks/src/architecture/
├── mod.rs                      the module tree
├── tests/                      the Tailwind-sources gate test over this checkout
├── workspace_law_locations.rs  the application packages, the Tailwind stylesheet and the frontend-layering configuration the laws read
└── workspace_laws.rs           the five workspace-law gates: print each library report
```

## How it works

Each gate reads source text from the working tree (untracked files included) with
`verification_core::scan`, compiles its matchers in, and treats an input it could not read as a
check that did not run, never as a pass.

| Verb | Reads | Fails when | Exit codes |
|---|---|---|---|
| `crate-tiers`, `crate-anatomy`, `test-file-reachability`, `frontend-layering`, `tailwind-sources` | the root `Cargo.toml` and every member manifest; the judged crates' sources; every member's module tree and test folders; the app's sources and the frontend crates' manifests; `crates/frontend/shell/frontend_application/style/aegis.css` | a law below is broken | 0 pass, 1 finding, 2 an input missing or unreadable |

### Workspace laws

Each of the five verbs prints the report of its law in
[`tools/foundation/repository_laws/src/workspace_laws/`](/tools/foundation/repository_laws/src/workspace_laws/README.md)
line for line and exits with its code. The paths and names that move with the tree are the
constants of `tools/checks/repository_checks/src/architecture/workspace_law_locations.rs`:
the application packages (`CRATE_TIERS`: `APPLICATION_PACKAGES` lists the five applications no
member may depend on, each a member; the crate-tier law's stray-manifest sweep reads the whole
checkout and takes no folder from here), the stylesheet, and the
frontend-layering configuration `FRONTEND_LAYERS`. That configuration has two halves:

- the in-crate half, `APP_LAYERS`: the layer table of `crates/frontend/shell/frontend_application`, which holds no module
  order any more (the foundation, the features, the pages and the workspaces are crates); the
  agent that births a crate out of a folder drops that folder's row or order in the same change;
- the crate-edge half, `FRONTEND_CRATE_EDGES`: the layer folders `crates/frontend/<layer>/`
  (`crates/frontend/shell` the shell layer), the app as the shell crate, and the crate orders
  `FOUNDATION_CRATE_ORDER`, `MISSION_CREATOR_CRATE_ORDER`, `DEBUG_BENCHES_CRATE_ORDER` and
  `SHELL_CRATE_ORDER` (the app and the offline service worker, peers that never name each
  other).

A crate an order names that no member carries is a finding. The `verify-workspace-laws` task row
runs the five laws in order as a step of `ci-local`.

## Public surface

- `workspace_laws::verify_crate_tiers`, `verify_crate_anatomy`, `verify_test_file_reachability`,
  `verify_frontend_layering` and `verify_tailwind_sources`: the five workspace-law gates over the
  checkout the command runs in; `workspace_law_report` and `verify_workspace_law` take a root.

## Boundaries

- Depends on: `repository_laws::workspace_laws` for the rules.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs`, for the five workspace-law verbs;
  - `tools/commands/ci_task_catalog/src/task_definitions.rs`, for the steps of
    `verify-workspace-laws`.
- Rules:
  - a law that could not read its input never reads as a pass.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the boundaries the
  workspace laws hold.
