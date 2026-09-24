**Status:** archived

# Relocation record: the tooling crates arrive under `tools_v2/`

What landed, and the measurements taken at the landing.

## What landed

- `tools_v2/verification-core`: the assertion, process, scan and lock library; package
  `verification-core`, Rust import `verification_core`.
- `tools_v2/ticket-engine`: the typed ticket models, canonical TOML, vocabulary, timestamps and
  transactional store; package `ticket-engine`, Rust import `ticket_engine`.
- `tools_v2/xtask`: the `xtask` package and binary, with its fixtures.
- Capture instructions live in [editor_capture.md](/documentation_v2/runbooks/editor_capture.md).

The Cargo alias, public domain types, serialized ticket data, dependency versions, dependency
checksums, binary names and the shared repository verification lock are all preserved. The lockfile
changes only the two package names and their dependency references.

## Relocation-sensitive behaviour

Workspace members, imports, manifest dependencies, compile-time repository roots, MCP fallback
paths, source-reading checks, blueprint fixtures, schema build inputs and source include discovery
all resolve through the new locations. The API's test-only `include_str!` of the deployment agent
points at the relocated source.

File-length verification requires each relocated crate and keeps its existing coverage. Deliberately
invalid citation fixtures are assembled at runtime, so the scanner can inspect its own source
without treating an example as a real contract. Missing roots still fail closed.

Runtime repository discovery and its test cwd lock are preserved. A regression test resolves
repository data from the root, the relocated crate and its source directory.

## Measurements

Run from the repository root:

| Command | Result |
|---|---|
| `cargo metadata --locked --no-deps --format-version 1` | passes |
| `cargo check --offline -p xtask -p verification-core -p ticket-engine -p ticketboard` | passes |
| `cargo check --locked -p website-api --tests` | passes, including the relocated embedded deployment source |
| `cargo test --locked --no-fail-fast -p verification-core -p ticket-engine -p xtask -p ticketboard` | verification library 68 passed, its doctest 1 passed; ticket library 72 passed; compile-fail integration test 1 passed; ticketboard 170 passed, 3 ignored |
| `cargo test --locked -p xtask -- --test-threads=1` | 831 passed, 4 ignored; the failure set matches the pre-move baseline exactly |
| `cargo xtask --help`, `ticket --help`, `map --help`, `schema list-gates` | pass from the root and from `tools_v2/xtask`; usage and command listing match the baseline |
| `cargo xtask ticket check` | passes before and after the move |
| `cargo xtask schema citations` | passes; all 98 citations resolve |
| `cargo xtask verify file-length` | passes |
| `cargo fmt --all --check` | passes |

Lockfile comparison confirms no dependency upgrades. File inventory comparison confirms every
original crate file is present at its new location.

One parallel run of the xtask suite failed a mod-compile test with an OS file-not-found error; it
passes in isolation and in the serial suite. The intermittent result is recorded rather than
weakening the assertion.
