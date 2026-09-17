# Tools V2 phase-two handoff

Phase two is implemented on `main`, starting from phase-one commit `c2c11b7b6`.
The existing working-tree changes are preserved. Phases three and four are not
implemented here.

## Live ownership and compatibility

- `developer-tools` is a workspace member at `tools_v2/developer-tools`; its Rust
  import is `developer_tools`. The previous `tools/tbd-tools` crate is absent.
- The six binary names remain `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`.
  Direct Cargo invocations select `-p developer-tools`. Existing `cargo xtask`
  command names, including the `ci tbd-tools-test` task alias, remain unchanged.
- `developer_tools::blueprint` owns the compiler, ingestion, parity reporting,
  profile discovery, and its test suite. Its command entry points receive the
  active repository path and return `anyhow::Result<u8>`.
- `developer_tools::map_verification` owns object goldens, labels, terrain
  manifests, BLAS manifests, and world line-of-sight verification. The terrain
  subsystem and its tests are extracted from the larger schema-gates module.
- `xtask` delegates through `commands/map` and `verifications/map_assets`. It has
  no direct map-engine dependency, including test dependencies. Map-engine remains
  a transitive dependency through `developer-tools`.
- Compiler fixtures live in `developer-tools/test_fixtures/blueprint`.
  Execution-receipt fixtures remain in `xtask/tests/fixtures`. Tests resolve the
  active checkout at runtime rather than embedding a compiler checkout path.
- Package selectors, source-reading checks, browser environment paths, CI filters,
  verification roots, build fingerprints, and operational documentation use the
  live crate. Historical ticket records and phase-one evidence remain historical.

## Shared archive implementation

`developer_tools::enfusion_pak` contains one bounded FORM/PAC1 directory parser and
one payload reader. Directory traversal is iterative, reads check bounds, and
entry offsets are absolute. Both duplicate `pak.rs` implementations are removed.

The explicit policies preserve the caller contracts:

- Blueprint access is case-insensitive, checks DATA spans, accepts zlib, checks
  decompressed lengths, and fails when an archive cannot be parsed.
- World access normalizes slashes without folding case, accepts zlib and raw
  deflate, exposes method/DATA-start metadata and raw bytes, and prints a diagnostic
  before skipping an invalid archive.
- Sorted archive order retains the first duplicate path. Loose-file resolution
  and ordered fallback are preserved; corruption in a selected source does not
  silently fall through to another source.

All new archive production modules are under 500 lines; their tests are separate
files under 1,000 lines. Existing relocated modules retain their size exemptions
with unchanged reasons and expiry dates. The two deleted parser exemptions are
removed. Broad module decomposition and inline-test extraction remain Phase 4.

## Validation

Baseline, before source changes:

- Heavy-tooling library: 125 passed.
- `xtask`: 831 passed, nine failed, four ignored. The nine failures match the
  phase-one handoff.

Final standard suite:

```text
cargo test --locked -p xtask -p developer-tools --no-fail-fast -- --test-threads=1
developer-tools: 233 passed, 0 failed, 4 ignored
xtask:          732 passed, 9 failed, 0 ignored
```

Test ownership moves account for the changed package totals. Nine new tests cover
archive policies/corruption, runtime fixture discovery, dependency direction, and
executable ownership. The final nine failures are exactly the baseline failure
set; no test assertions or golden expectations are weakened.

Additional checks pass:

- `cargo check --locked -p xtask -p developer-tools -p verification-core
  -p ticket-engine -p ticketboard --all-targets`.
- `cargo build --locked -p developer-tools --bins -p xtask`.
- `cargo metadata --locked --no-deps --format-version 1`, including assertions
  about package ownership and dependency direction.
- `cargo fmt --all --check` and whitespace checks over the Phase 2 changes.
- `cargo xtask schema citations`: all 98 citations resolve.
- `cargo xtask verify file-length`: no violations.
- Ticket checking, BLAS manifest verification, map-object goldens, Everon terrain
  manifest verification, height labels, and locations.
- CLI help for `xtask`, map commands, blueprint compilation, and the five
  help-enabled binaries; the sixth binary, `mcpd`, passes its offline stub check.
- The Cargo alias works from `tools_v2/xtask`. Archive dry-runs work from that
  directory, `tools_v2/developer-tools`, and its nested blueprint source directory.
- All 30 relocated blueprint/prefab/world-parity fixture files and 59 existing
  heavy-tooling configuration/fixture files are byte-identical. External dependency
  versions and checksums are unchanged. Initial modified-file hashes still match.

The four normally ignored local-asset tests were also explicitly run. Three pass:
PAK census, farmhouse XOB bytes versus the loose extract, and XOB socket/material
decoding. The farmhouse closure test fails on the local asset version: shell
triangle counts are `(2883, 0, 0)` while the existing expectation is `(4012, 0, 0)`.
The preserved pre-refactor `xtask` test executable reproduces that exact failure
against the same 222,566 files in 16 archives. The expectation is unchanged.

## Existing blockers

The standard suite retains these baseline failures:

1. Three `gate_t437` tests and four objective-related schema tests refer to the
   absent mod `Scripts/Game/TBD/Objectives` location.
2. The host/container bridge test expects unequal glibc versions, but both are
   2.43 on this machine.
3. The unread-wire-field test finds `seats` and `area` counts different from its
   existing pins.

`cargo check --workspace --locked` still fails on frontend unresolved imports
`menu_geometry::menu_axis_position` and `menu_geometry::menu_scroll_top`, matching
the phase-one result. The separately exercised local-asset closure failure above
also predates this refactor. These prevent claiming that the entire repository is
green; they are not Phase 2 regressions.

The initial working-tree snapshot, baseline and final test logs, command logs,
fixture hashes, and dependency metadata are in `/tmp/tbd-tools-phase-two/` locally.
This handoff retains the results independently of those temporary files.
