# Frontend test support

The `frontend_test_support` crate: the helpers the tests of every frontend crate share. It reads
repository files from the root the workspace's one finder,
[`repository_root`](/crates/foundation/repository_root/README.md), answers (the captured
[API](/documentation/glossary/a_to_f.md#api) responses, the contract schemas, the API route
tables), lists the `src/` folder of every frontend package once for the scans that read the whole
frontend, scrubs a source text down to what a build compiles and runs, joins the shards a source pin
reads, and guards `view!` attribute values. Every frontend crate names it under
`[dev-dependencies]` only, so no shipped build links it.

## Contents

```text
crates/frontend/foundation/frontend_test_support/
├── Cargo.toml  the package: `repository_root`, layout tier 1, any target
└── src/        the repository file reads, `golden!`, the frontend source roots, the scrubber, the source shards and the attribute guard
```

## How it works

A test passes its own `env!("CARGO_MANIFEST_DIR")`, because `env!` expands in the crate that
spells it; `repository_root` walks up from there to the folder holding the `.ai/tickets/ROOT`
marker, so a test reads the same repository file at whatever depth its crate sits.
`golden!("GET__me.json")` expands that argument in the calling crate and reads the captured
response from that root. The source pins of each area embed their own files with
`include_str!` and read them through the scrubber, so a mention in a comment, a literal or code no
build reaches never satisfies a pin. The [source tree README](src/README.md) describes each
module.

## Getting started

Run from the repository root:

```bash
cargo test -p frontend_test_support   # the scrubber, the repository file reads and the attribute guard
```

A frontend crate adds `frontend_test_support = { workspace = true }` to its
`[dev-dependencies]` and imports `frontend_test_support::prelude::*` or the module it needs.

## Configuration

None: no feature, no environment variable. The crate reads files at test run time only.

## Public surface

- `golden!` (crate root, also `fixtures::golden` and the prelude), `fixtures::{golden_text,
  FIXTURE_DIR, api_route_source, crate_cargo_toml, strip_toml_comments}`: the captured responses,
  the API route tables and the calling crate's manifest.
- `repository_root::{repository_root, repository_path, repository_text, source_file_folder,
  cached_text}`: the root of the calling crate's checkout and the cached reads of repository
  files.
- `frontend_source_roots::{frontend_source_roots, assert_each_package_read_once,
  FRONTEND_CRATE_MEMBER_GLOB}`: the `src/` folder of every frontend package, one per package, and
  the check that a scan's root list names each package once.
- `class_r_scrub::{live_source, live_code, only_item, only_body}`: the scrubbed views of a text.
- `source_shards::{production_source, production_shard}`: the joined text of a pinned file.
- `view_attribute_guard::{assert_view_attributes_are_well_formed, view_attribute_findings}`: the
  `view!` attribute guard.
- `prelude`: the scrubbed views, `golden!`, `repository_path`, `repository_text` and the shard
  joins.

## Boundaries

- Depends on: `repository_root` (the checkout-root walk); at test run time, the repository files a
  test names and the calling crate's `Cargo.toml`.
- Used by: the tests of the single-page app (`crates/frontend/shell/frontend_application`) and of every crate under
  `crates/frontend/`, through `[dev-dependencies]`.
- Rules: no dependency on `apps/` and no shipped dependent (`cargo xtask ci
  verify-workspace-laws`: the frontend crate order lists it as reached only through
  dev-dependencies); no path here counts parent-folder steps from a crate or a file; a
  whole-frontend scan reads each package once.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md#shared-foundations) — the shared
  foundations whose tests this crate serves.
