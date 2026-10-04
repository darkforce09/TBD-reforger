# Frontend crates

The library crates of the single-page app (`apps/frontend`), one folder per layer:
`crates/frontend/<layer>/<name>`, where the package name equals the folder name. The app links
them and keeps only its entry point, its route rendering and its platform frame.

## Contents

```text
crates/frontend/
├── features/    the product capabilities more than one page or workspace shows
├── foundation/  the zero-business-logic building blocks every frontend layer above reads
├── pages/       the platform pages, one crate per sidebar section
└── workspaces/  the full-screen applications: the Mission Creator's crates and the debug benches
```

## How it works

The layers keep the app's order: foundation, then features, then pages and workspaces, then the
app itself as the shell. A crate depends only on crates of a lower layer, or on lower crates of
its own layer's order; page crates never depend on each other. Every crate declares
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
- Used by: the single-page app `apps/frontend`, and the frontend crates of higher layers.
- Rules: every edge between frontend crates follows the layer order and the crate orders of the
  frontend-layering law (`cargo xtask ci verify-workspace-laws`); a crate that depends on leptos
  has its own `@source` line in `apps/frontend/style/aegis.css`; a dev-dependency never points at
  `apps/`.

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md) — the app these crates serve.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
