# Test support

The source tree of `frontend_test_support`, the helpers the frontend crates' tests share: the
repository files read from the checkout root `repository_root` finds (the captured
[API](/documentation/glossary/a_to_f.md#api) responses, the contract schemas, the API route
tables), the `src/` folders of the frontend packages, the source scrubber, the joining of the
shards a source pin reads, and the `view!`
attribute guard several page areas run over their own folders. Everything here is test-only.

## Contents

```text
crates/frontend/foundation/frontend_test_support/src/
├── class_r_scrub/            the source scrubber the source pins read production code through
├── fixtures.rs               `golden!`, the captured responses, the API route tables, `Cargo.toml`
├── frontend_source_roots.rs  the `src/` of every frontend package, once each, for whole-frontend scans
├── lib.rs                    the module tree
├── prelude.rs                the helpers most tests import
├── repository_root.rs        the calling crate's checkout root and the cached reads of repository files
├── source_shards.rs          joins the shards of one logical source file for a source pin
└── view_attribute_guard.rs   the source guard over `view!` attribute values
```

## How it works

Every frontend crate names this crate under `[dev-dependencies]`, so no shipped build compiles
it.

- `repository_root.rs`: `repository_root(manifest_dir)` is
  `repository_root::find_repository_root_from` from the caller's `env!("CARGO_MANIFEST_DIR")`, the
  nearest folder holding the `.ai/tickets/ROOT` marker, and panics with the walk's error (the
  folder searched from and the marker) when there is none. `repository_text` and `repository_path` address a file by its path from that root, so a
  test reads the same file whatever the depth of its crate; `repository_text` reads each file once
  per test process and hands out `&'static str`. `source_file_folder(manifest_dir, file!())` is the
  folder of the calling source file, for a test that scans the folders around it. The caller
  passes `env!("CARGO_MANIFEST_DIR")` because `env!` expands in the crate that spells it.
- `fixtures.rs`: `golden!("GET__me.json")` forwards the calling crate's manifest folder to
  `golden_text`, which reads one captured response from `contracts/fixtures/api_goldens/`
  (`FIXTURE_DIR`) under that root; `api_route_source` concatenates the eight domain route
  tables of the API, one named path per domain, for guards that assert a frontend call has a route
  behind it; `crate_cargo_toml(manifest_dir)` returns the calling crate's own manifest, and
  `strip_toml_comments` removes its comments, for guards on a declared dependency or feature.
- `frontend_source_roots.rs`: `frontend_source_roots(repository)` lists the `src/` of every crate
  folder the member glob `crates/frontend/*/*` reaches, the shell layer's app and offline service
  worker included, sorted; it fails closed on a missing member glob, a crate folder without its
  `Cargo.toml` or `src/`, or an unreadable folder. `assert_each_package_read_once(roots)` keys
  each root by the `[package]` name of the manifest beside it and panics when one package appears
  twice, under one spelling or two; the documentation audit of the app and the Mission Creator's
  whole-frontend pins each run it over their own root list.
- `source_shards.rs`: `production_source` joins the shards of one logical source file in the order
  given, each through `production_shard`, which first drops its `#[cfg(test)] #[path = …] mod …;`
  lines, because the scrubber cuts from the first `#[cfg(test)]` and would otherwise hide every
  shard after the first. The pins themselves live with the code they read: each area's
  `tests/source_pins.rs` embeds its own files with `include_str!` relative to the pin file.
- `view_attribute_guard.rs`: `assert_view_attributes_are_well_formed` reads every production
  source under the folders a test passes and refuses a `view!` attribute value that ends its tag
  early or goes unbraced; `view_attribute_findings` is the scan over one text.
- `class_r_scrub/`: reduces a text to what a build compiles and runs.

## Boundaries

- Depends on: `repository_root` for the root; files read at test run time from it: the fixture corpus in
  `contracts/fixtures/api_goldens/`, the route tables `routes.rs` of the eight API domain crates
  under `crates/api/`, and whatever repository path a test names; the calling crate's
  `Cargo.toml`; the production folders a test hands the attribute guard.
- Used by: the unit tests of the frontend crates and the app: the golden round trips of the DTOs,
  the route checks of the endpoints, the source pins of every area (`tests/source_pins.rs`), and
  the tests of the pages and of the Mission Creator.
- Rules: a dev-only crate, so no shipped code can depend on it; no path here counts parent-folder
  steps from a crate or a file, so a crate moves without touching a test; a joined source keeps
  each shard once, which keeps the one-definition guarantee `only_item` and `only_body` rely on;
  the eight route tables stay enumerated one by one, so a domain renamed in the API fails every
  guard that reads them.

## Related documentation

- [Frontend test support](/crates/frontend/foundation/frontend_test_support/README.md) — the
  crate, its public surface and how a crate adds it.
