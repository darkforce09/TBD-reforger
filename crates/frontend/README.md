# Frontend crates

The crates of the single-page app, one folder per layer: `crates/frontend/<layer>/<name>`, where
the package name equals the folder name. The library crates fill the four lower layers; the top
layer, the shell, holds the app itself (`crates/frontend/shell/frontend_application`), which links
them and keeps only its entry point, its route rendering and its platform frame, and the offline
service worker beside it.

## Contents

```text
crates/frontend/
├── features/    the product capabilities more than one page or workspace shows
├── foundation/  the zero-business-logic building blocks every frontend layer above reads
├── pages/       the platform pages, one crate per sidebar section
├── shell/       the top layer: the single-page app binary and the offline service worker binary
└── workspaces/  the full-screen applications: the Mission Creator's crates and the debug benches
```

## How it works

The layers keep one order: foundation, then features, then pages and workspaces, then the shell,
where the app and the offline service worker are peers that never depend on each other. A crate
depends only on crates of a lower layer, or on lower crates of its own layer's order; page crates
never depend on each other. Every crate declares
`[package.metadata.layout]` with the category `crates/frontend/<layer>`, its tier and its targets;
the browser crates sit in a `cfg(target_arch = "wasm32")` dependency table, so every crate
compiles natively for its tests and to wasm32 for the bundle.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_test_support            # one crate's unit tests
cargo xtask ci verify-workspace-laws           # tiers, anatomy, frontend layering, Tailwind sources
```

## Boundaries

- Depends on: the library crates under `crates/` except the API's, and external crates from the
  root `[workspace.dependencies]`.
- Used by: the single-page app `crates/frontend/shell/frontend_application`, and the frontend
  crates of higher layers.
- Rules: every edge between frontend crates follows the layer order and the crate orders of the
  frontend-layering law (`cargo xtask ci verify-workspace-laws`); a crate that depends on leptos
  has its own `@source` line in `crates/frontend/shell/frontend_application/style/aegis.css`; no
  crate depends on a shell crate, in any table, dev-dependencies included.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  the app these crates serve.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
