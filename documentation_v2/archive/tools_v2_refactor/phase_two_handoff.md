# Consolidation record: the heavy services land in `developer-tools`

What landed, and the measurements taken at the landing.

## Ownership

- `developer-tools` is a workspace member at `tools_v2/developer-tools`; its Rust import is
  `developer_tools`. It is the only heavy tooling crate in the workspace.
- The six binary names are `enf`, `gate`, `mcpd`, `world`, `map` and `capture`. Direct Cargo
  invocations select `-p developer-tools`; the library-test task is `ci developer-tools-test`.
- `developer_tools::blueprint` owns the compiler, ingestion, parity reporting, profile discovery
  and its test suite. Its command entry points receive the active repository path and return
  `anyhow::Result<u8>`.
- `developer_tools::map_verification` owns object goldens, labels, terrain manifests, BLAS
  manifests and world line-of-sight verification.
- `xtask` delegates through `commands/map` and `verifications/map_assets`. It has no direct
  map-engine dependency, including test dependencies; the map engine is transitive through
  `developer-tools`.
- Compiler fixtures live in `developer-tools/test_fixtures/blueprint`. Tests resolve the active
  checkout at runtime rather than embedding a compiler checkout path.

## One archive reader

`developer_tools::enfusion_pak` contains one bounded FORM/PAC1 directory parser and one payload
reader. Directory traversal is iterative, reads check bounds, and entry offsets are absolute. The
explicit policies preserve each caller's contract:

- Blueprint access is case-insensitive, checks DATA spans, accepts zlib, checks decompressed
  lengths, and fails when an archive cannot be parsed.
- World access normalises slashes without folding case, accepts zlib and raw deflate, exposes
  method and DATA-start metadata and raw bytes, and prints a diagnostic before skipping an invalid
  archive.
- Sorted archive order retains the first duplicate path. Loose-file resolution and ordered fallback
  are preserved; corruption in a selected source does not silently fall through to another source.

## Measurements

```text
cargo test --locked -p xtask -p developer-tools --no-fail-fast -- --test-threads=1
developer-tools: 233 passed, 4 ignored
xtask:           732 passed
```

Test ownership moves account for the changed package totals. Nine tests cover archive policies and
corruption, runtime fixture discovery, dependency direction and executable ownership. No test
assertion or golden expectation is weakened.

| Check | Result |
|---|---|
| `cargo check --locked -p xtask -p developer-tools -p verification-core -p ticket-engine -p ticketboard --all-targets` | passes |
| `cargo build --locked -p developer-tools --bins -p xtask` | passes |
| `cargo metadata --locked --no-deps --format-version 1` | passes, including package ownership and dependency direction |
| `cargo fmt --all --check` | passes |
| `cargo xtask schema citations` | all 98 citations resolve |
| `cargo xtask verify file-length` | no violations |
| Ticket checking, BLAS manifest, map-object goldens, Everon terrain manifest, height labels, locations | pass |
| CLI help for `xtask`, the map commands and the five help-enabled binaries | pass; `mcpd` passes its offline stub check |
| The Cargo alias from `tools_v2/xtask`, and archive dry-runs from three directories | pass |

All 30 relocated blueprint, prefab and world-parity fixture files and 59 configuration and fixture
files are byte-identical. External dependency versions and checksums are unchanged.

The four normally ignored local-asset tests were run explicitly. Three pass: the PAK census, the
farmhouse XOB bytes against the loose extract, and XOB socket and material decoding. The farmhouse
closure test fails on the local asset version — shell triangle counts are `(2883, 0, 0)` against an
expectation of `(4012, 0, 0)` — over the same 222,566 files in 16 archives. The expectation is
unchanged.
