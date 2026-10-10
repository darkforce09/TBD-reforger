# Frontend test support

The `frontend_test_support` crate: the helpers the tests of every frontend crate share. It reads
repository files from the root the workspace's one finder,
[`repository_root`](/crates/foundation/repository_root/README.md), answers (the captured
[API](/documentation/glossary/a_to_f.md#api) responses, the contract schemas, the API route
tables). Every frontend crate names it under `[dev-dependencies]` only, so no shipped build links
it.

## Contents

```text
crates/frontend/foundation/frontend_test_support/
├── Cargo.toml  the package: `repository_root`, layout tier 1, any target
└── src/        the repository file reads, `golden!` and the API route tables
```

## How it works

A test passes its own `env!("CARGO_MANIFEST_DIR")`, because `env!` expands in the crate that
spells it; `repository_root` walks up from there to the folder holding the `.ai/ROOT`
marker, so a test reads the same repository file at whatever depth its crate sits.
`golden!("GET__me.json")` expands that argument in the calling crate and reads the captured
response from that root. The [source tree README](src/README.md) describes each module.

## Getting started

A frontend crate adds `frontend_test_support = { workspace = true }` to its
`[dev-dependencies]` and imports `frontend_test_support::prelude::*` or the module it needs.

## Configuration

None: no feature, no environment variable. The crate reads files at test run time only.

## Public surface

- `golden!` (crate root, also `fixtures::golden`), `fixtures::{golden_text, FIXTURE_DIR,
  api_route_source}`: the captured responses and the API route tables.
- `repository_root::{repository_root, repository_path, repository_text, cached_text}`: the root
  of the calling crate's checkout and the cached reads of repository files.
- `prelude`: `golden!`, `repository_path` and `repository_text`.

## Boundaries

- Depends on: `repository_root` (the checkout-root walk); at test run time, the repository files a
  test names.
- Used by: the tests of the single-page app (`crates/frontend/shell/frontend_application`) and of
  every crate under `crates/frontend/`, through `[dev-dependencies]`.
- Rules: no dependency on an application crate and no shipped dependent (`cargo xtask ci
  verify-workspace-laws`: the frontend crate order lists it as reached only through
  dev-dependencies); no path here counts parent-folder steps from a crate or a file.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md#shared-foundations) — the shared
  foundations whose tests this crate serves.
