# Tools V2 phase-one handoff

Phase-one implementation is complete on `main`. The chosen live destination is
`tools_v2/`. Validation reproduces the existing workspace and test failures listed
below; those failures prevent a claim that the whole workspace is green.

## Live layout and compatibility

- `tools_v2/verification-core`: the existing assertion, process, scan, and lock
  library; package `verification-core`, Rust import `verification_core`.
- `tools_v2/ticket-engine`: the existing typed ticket models, canonical TOML,
  vocabulary, timestamps, and transactional store; package `ticket-engine`, Rust
  import `ticket_engine`.
- `tools_v2/xtask`: the existing `xtask` package and binary, including its flat
  modules, blueprint compiler, ticket orchestration, and `tests/fixtures`.
- `tools/tbd-tools`: the existing heavy CLI, with its current package and binary
  names. `tools_v2/developer-tools` remains a documentation scaffold outside the
  Cargo workspace.
- Root `crates/` and `xtask/`, plus `tools/editor-capture/` and the orphaned
  `tools/pbo/` bytecode directory, are absent. Capture instructions live in
  [editor_capture.md](../docs/tools/editor_capture.md).

The Cargo alias, public domain types, serialized ticket data, dependency versions,
dependency checksums, binary names, and the shared repository verification lock are preserved.
Cargo.lock changes only the two package names and their dependency references.
Existing working-tree changes were retained during relocation. The phase-one
commit excludes the pre-existing edits to ticket checking, status markers, view
synchronization, mod-compiler comments, and the full CLAUDE.md rewrite; those edits
remain local at their current paths. Validation below describes the working
tree with those existing edits present. No branch was created.

## Relocation-sensitive behavior

Workspace members, imports, manifest dependencies, compile-time repository roots,
MCP fallback paths, source-reading checks, blueprint fixtures, schema build inputs,
and source include discovery use the new locations. The API's test-only
`include_str!` of the deployment agent points at the relocated source too.

Citation scanning covers `apps`, `packages`, `tools`, and `tools_v2`. File-length
verification requires each of the three relocated crates and retains the existing
tools and website coverage. Existing size exemptions retain their original reasons
and expiry dates. Deliberately invalid citation test fixtures are assembled at
runtime so the scanner can inspect its own source without treating examples as
real contracts. Tests continue to prove missing roots fail closed.

Runtime repository discovery and its test cwd lock are preserved. A new regression
test resolves repository data from the root, the relocated crate, and its source
directory. Historical ticket records and the one-shot migration's historical
ownership-path inputs retain their original spellings.

## Validation results

Run from the repository root unless stated otherwise:

- `cargo metadata --locked --no-deps --format-version 1`: passes.
- `cargo check --offline -p xtask -p verification-core -p ticket-engine -p ticketboard`:
  passes.
- The staged phase-one snapshot also passes the tooling package check after
  excluding the pre-existing working-tree edits from the commit.
- `cargo check --locked -p website-api --tests`: passes, including the relocated
  embedded deployment source.
- `cargo test --locked --no-fail-fast -p verification-core -p ticket-engine -p xtask -p ticketboard`:
  verification library 68 passed; verification doctest 1 passed; ticket library
  72 passed and 1 pre-existing failure; compile-fail integration test 1 passed;
  ticketboard 170 passed and 3 ignored. Ticket-engine has no doctests.
- Final `cargo test --locked -p xtask -- --test-threads=1`: 831 passed, 9 failed,
  4 ignored. Its failure set exactly matches the baseline's 9 failures; the
  baseline had 830 passes before the repository-root regression test was added.
- `cargo xtask --help`, `cargo xtask ticket --help`, `cargo xtask map --help`, and
  `cargo xtask schema list-gates`: pass from both the root and `tools_v2/xtask`.
  Top-level CLI usage and command listing match the baseline.
- `cargo xtask ticket check`: passes before and after relocation.
- `cargo xtask schema citations`: passes; all 98 citations resolve across both
  tooling trees and the other configured roots.
- `cargo xtask verify file-length`: passes with the original exemption dates.
- `cargo fmt --all --check`: passes.
- Lockfile comparison confirms no dependency upgrades. File inventory comparison
  confirms every original crate file is present at its new location. Operational
  source references no longer require the removed directories.
- `git diff --check` reports only an existing extra EOF blank line in
  `apps/mod/tbd-export/README.md`; that file is unchanged by this work. Relocated
  source and new documentation have no trailing whitespace.

The first parallel post-move xtask test run additionally failed
`gate_mod_compile::tests::no_server_is_rc3` with an OS file-not-found error. It
passes in isolation and in the final serial suite. This intermittent result is
recorded rather than changing the mod compiler or weakening its assertion.

## Existing blockers

These were observed before relocation and remain afterward:

1. `cargo check --workspace --locked`: frontend unresolved imports
   `menu_geometry::menu_axis_position` and `menu_geometry::menu_scroll_top`.
2. `ticket_engine::store::tests::corpus_roundtrip_real_tree_byte_identical`:
   ticket T-129 contains `priority = 3`, which canonical rendering omits.
3. Three `gate_t437` tests and four objective-related `schema_gates::t212_*`
   tests expect the absent mod `Scripts/Game/TBD/Objectives` location.
4. `hostrun::tests::the_bridge_really_crosses_the_container_wall`: host and
   container both report glibc 2.43, violating the test's inequality assertion.
5. `schema_gates::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree`:
   `seats` and `area` identifier counts differ from their existing pins.

Detailed baseline and post-move logs and the original working-tree snapshot are
available locally under `/tmp/tbd-tools-phase-one/`. This document preserves the
results independently of those temporary files.

## Starting phase two

Use [ARCHITECTURE_PLAN.md](./ARCHITECTURE_PLAN.md) with `tools_v2/` as the target.
The next phase must first migrate the live `tools/tbd-tools` implementation into
the `developer-tools` scaffold and reconnect consumers, then relocate the blueprint
compiler and its fixtures and unify archive readers. The scaffold manifests and
README module trees describe the final architecture; do not treat them as already
implemented. Ticket consolidation and broad module/test decomposition remain in
phases three and four.
