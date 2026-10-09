# Test support

The source tree of `frontend_test_support`, the helpers the frontend crates' tests share: the
repository files read from the checkout root `repository_root` finds (the captured
[API](/documentation/glossary/a_to_f.md#api) responses, the contract schemas, the API route
tables). Everything here is test-only.

## Contents

```text
crates/frontend/foundation/frontend_test_support/src/
├── fixtures.rs         `golden!`, the captured responses, the API route tables
├── lib.rs              the module tree
├── prelude.rs          the helpers most tests import
└── repository_root.rs  the calling crate's checkout root and the cached reads of repository files
```

## How it works

Every frontend crate names this crate under `[dev-dependencies]`, so no shipped build compiles
it.

- `repository_root.rs`: `repository_root(manifest_dir)` is
  `repository_root::find_repository_root_from` from the caller's `env!("CARGO_MANIFEST_DIR")`, the
  nearest folder holding the `.ai/tickets/ROOT` marker, and panics with the walk's error (the
  folder searched from and the marker) when there is none. `repository_text` and
  `repository_path` address a file by its path from that root, so a test reads the same file
  whatever the depth of its crate; `repository_text` reads each file once per test process and
  hands out `&'static str`. The caller passes `env!("CARGO_MANIFEST_DIR")` because `env!` expands
  in the crate that spells it.
- `fixtures.rs`: `golden!("GET__me.json")` forwards the calling crate's manifest folder to
  `golden_text`, which reads one captured response from `contracts/fixtures/api_goldens/`
  (`FIXTURE_DIR`) under that root; `api_route_source` concatenates the eight domain route
  tables of the API, one named path per domain, for the check that a frontend call has a route
  behind it.

## Boundaries

- Depends on: `repository_root` for the root; files read at test run time from it: the fixture
  corpus in `contracts/fixtures/api_goldens/`, the route tables `routes.rs` of the eight API
  domain crates under `crates/api/`, and whatever repository path a test names.
- Used by: the unit tests of the frontend crates and the app: the golden round trips of the DTOs,
  the route check of the endpoints, and the tests of the pages and of the Mission Creator.
- Rules: a dev-only crate, so no shipped code can depend on it; no path here counts parent-folder
  steps from a crate or a file, so a crate moves without touching a test; the eight route tables
  stay enumerated one by one, so a domain renamed in the API fails every check that reads them.

## Related documentation

- [Frontend test support](/crates/frontend/foundation/frontend_test_support/README.md) — the
  crate, its public surface and how a crate adds it.
