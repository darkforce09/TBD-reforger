# Test support

The helpers the crate's own tests share: the source scrubber, the captured
[API](/documentation_v2/glossary.md#api) responses, the text of production files for source pins,
and the text of the files outside `apps/website/frontend/src/` that the guard tests read.
Everything here is test-only and resolved at compile time.

## Contents

```text
apps/website/frontend/src/v2/core/test_support/
├── class_r_scrub/        the source scrubber the source pins read production code through
├── editor_operations.rs  Mission Creator and map-engine operation sources, as text
├── fixtures.rs           embeds the captured API responses, the API route tables and `Cargo.toml`
├── mod.rs                the module tree, compiled only in test builds
├── pins.rs               one function per logical production file, its shards concatenated
└── tests/                unit tests for the source scrubber
```

## How it works

`fixtures.rs`, `pins.rs` and `editor_operations.rs` embed files with `include_str!`, so a test
reads production text and fixtures without touching the file system at run time, and a path that
stops existing breaks the test build rather than passing quietly. The paths are anchored on the
crate manifest or on the helper file itself, never on the calling test, so a test moves freely
within `apps/website/frontend/src/` and a production file that moves changes one path here.

- `fixtures.rs`: the `golden!` macro embeds one captured response from
  `apps/website/frontend/tests/fixtures/api/` by file name (`golden!("GET__me.json")`);
  `api_route_source` concatenates the eight domain route tables of the API, one named
  `include_str!` per domain, for guards that assert a frontend call has a route behind it;
  `crate_cargo_toml` returns this crate's manifest, for guards on a declared dependency.
- `pins.rs`: `client_source`, `sse_source`, `ui_source`, `auth_source` and one function for each
  page whose tests pin its source return the full text of a logical source file, concatenating
  its shards in declaration order. Each shard loses its `#[cfg(test)] #[path = …] mod …;` lines
  first, because the scrubber cuts from the first `#[cfg(test)]` and would otherwise hide every
  shard after the first.
- `editor_operations.rs`: `ENTITY`, `CONTEXT` and `DOMAIN_ENTITY` hold the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s host-state shards (armed
  placement, entity selection, the installed editor context) and the map engine's hosted document
  commands and entity operations, as the structural checks of the editor's tests read them.
- `class_r_scrub/`: reduces such a text to what a build compiles and runs.

## Public surface

All `pub(crate)`, for the crate's tests alone:

- `class_r_scrub::{live_source, live_code, only_item, only_body}`: the scrubbed views of a text.
- `fixtures::golden!`, `fixtures::FIXTURE_DIR`, `fixtures::api_route_source` and
  `fixtures::crate_cargo_toml`: the embedded fixtures and cross-crate files.
- `pins::*_source`: the concatenated text of the API client, the live status stream, the UI
  primitives, the auth modules and the page sources their tests pin.
- `editor_operations::{ENTITY, CONTEXT, DOMAIN_ENTITY}`: the editor and engine operation sources.

## Boundaries

- Depends on: files read as text at compile time: the fixture corpus in
  `apps/website/frontend/tests/fixtures/api/`; the route tables `routes.rs` of the eight domains
  under `apps/website/api_v2/src/`; the Mission Creator's
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`; the map engine's
  `apps/website/map-engine/src/editing/hosted_commands/` and
  `apps/website/map-engine/src/data/store/operations/entity/`; and the production files of this
  crate that `pins.rs` names.
- Used by: the unit tests of the whole crate: the golden round trips in
  `apps/website/frontend/src/v2/core/api/dto/tests/`, the route checks in
  `apps/website/frontend/src/v2/core/api/endpoints/tests/`, the source pins of the core modules,
  and the tests of the pages under `apps/website/frontend/src/v2/pages/` and of the Mission
  Creator under `apps/website/frontend/src/v2/apps/editor/`.
- Rules: `apps/website/frontend/src/v2/core/mod.rs` declares this module under `#[cfg(test)]`, so
  no shipped code can depend on it; a pin lists each shard once, which keeps the one-definition
  guarantee `only_item` and `only_body` rely on; the eight route tables stay enumerated one by
  one, so a domain renamed in the API breaks this crate's test build.
