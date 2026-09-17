# Tools V2 phase-three handoff

Phase three consolidates the ticket subsystem on `main`, following phase-two
commit `c3c75fc33`. Phase four is not implemented.

## Ownership and interfaces

`ticket-engine` owns validation, command services, generated views, registry
compatibility, ticket maintenance, wave scheduling/history, receipts, and estimates.
Its manifest has no workspace dependencies. The public ticket model, `Corpus`,
`ops`, encoding, and vocabulary exports remain compatible with ticketboard.

- Validation preserves schema/rule order, strict checks, diagnostics, and refusal
  behavior. Live writes still use typed operations and surgical corpus persistence.
- Registry compatibility readers retain legacy formats and their writer restrictions.
  Historical status lookup, cached shipping status, and permissive display-title
  lookup belong to the same crate.
- Synchronization retains all six Markdown views, queue JSON, roadmap markers, and
  gap-analysis updates. The existing removal of CLAUDE.md status synchronization is
  preserved.
- Ticket CLI services receive the active root. Batch selection invokes a supplied
  execution callback; cleanup target resolution returns the path and branch without
  deleting them.
- `xtask::commands::ticket` owns agent invocation and worktree/branch cleanup.
  `ticket done` retains cleanup-before-shipping order. The slice runner writes
  receipts through ticket-engine.
- `xtask::commands::wave` delegates lock and collision commands. The platform wave
  driver remains in xtask and shares ticket-engine's close-marker/history rules.
  HEAD inclusion, disavowed markers, highest versus newest claims, shallow-history
  refusal, and historical plan decoding retain their existing semantics.

Production files in ticket-engine are at most 379 lines; test files are at most
757 lines. Inline tests are extracted into sibling test directories. Thirteen
obsolete size exemptions are removed, without new exemptions or extended expiry
periods. All existing test functions remain; nine tests cover the new execution,
cleanup, and dependency boundaries.

The three execution-receipt fixtures live in
`ticket-engine/tests/fixtures/execution_receipts`. Both crates resolve them from
the active checkout; their bytes match the original fixtures. External package
versions, sources, and checksums are unchanged. Five existing external dependencies
are added to ticket-engine, and xtask's unused direct `time` dependency is removed.

## Verification

The standard command is:

```text
cargo test --locked -p ticket-engine -p xtask -p ticketboard --no-fail-fast -- --test-threads=1
```

Baseline:

- Ticket-engine unit tests: 72 passed, 1 failed.
- Ticket-engine compile-fail contract: 1 passed.
- Xtask: 732 passed, 9 failed.
- Ticketboard: 170 passed, 0 failed, 3 ignored.

After consolidation:

- Ticket-engine unit tests: 212 passed, 1 failed.
- Ticket-engine compile-fail contract: 1 passed.
- Xtask: 601 passed, 9 failed.
- Ticketboard: 170 passed, 0 failed, 3 ignored.

The 135 relocated tests and nine additions explain the totals. No existing test
is removed, ignored, or weakened. Exhaustive matches preserve ticket-engine's
crate-level wildcard-match prohibition; lint cleanups preserve the existing
conditions and stable ordering.

Additional checks:

- `cargo check --locked -p ticket-engine -p xtask -p ticketboard --all-targets`.
- `cargo clippy --locked -p ticket-engine --all-targets -- -D warnings`.
- `cargo fmt --all --check`.
- `cargo doc --locked -p ticket-engine --no-deps`.
- Normal and strict ticket checks, wave checks, citation checks, and file-length checks.
  Strict checking also passes with an isolated Git index containing every new file;
  the real working index is unchanged.
- CLI help and nested-directory invocation, including the Cargo alias.
- Dependency assertions reject internal workspace dependencies from ticket-engine
  and duplicate ticket implementations in xtask.

A retained pre-refactor executable supplies the behavior oracle. Comparisons use
identical inputs and the same executable name for Clap help:

- Normal/strict ticket checks and wave checks match exit code, stdout, and stderr.
- Successful `wave repack` and `ticket set-status T-129 review` commands match
  output and all file bytes on the disposable copy; unknown-ticket removal
  refuses without changing files.
- Help, ticket queries, unknown IDs, invalid status changes, metrics, collision
  refusals, and dry-run execution match.
- Malformed schemas/tickets, missing vocabulary/lock, and invalid lock versions
  reproduce the same nonzero exits and diagnostics.
- Synchronization on a disposable live-corpus copy matches generated file bytes,
  changed-file sets, and command output. A second run is idempotent. The only
  generated file differing from that copy's initial contents is TICKET_REGISTRY.md;
  both executables produce the same change. Live generated views are untouched.
- Existing deterministic fixture tests retain mutation/refusal, child shipping,
  SHA stamping, no-repack, dependency packing, reservation, empty-wave, history,
  timestamp, receipt, and estimate coverage.

## Reproduced baseline failures

The entire repository is not green. These failures occur before and after this
refactor:

1. `store::tests::corpus_roundtrip_real_tree_byte_identical`: T-129 contains
   `priority = 3`, which the existing canonical renderer omits. The mismatch is
   at byte 355 in both runs. Ticket data and the assertion are unchanged.
2. Three `gate_t437` tests and four objective schema tests reference the absent
   mod `Scripts/Game/TBD/Objectives` location.
3. `hostrun::tests::the_bridge_really_crosses_the_container_wall` expects different
   host/container glibc versions; both are 2.43 here.
4. `schema_gates::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree`
   reports the existing `seats` and `area` identifier-count mismatches.

The initial working-tree diff and hashes, retained executable, test logs,
command comparisons, fixture hashes, and structural audit are stored locally at
`/tmp/tbd-tools-phase-three/`. Unrelated initial edits are preserved byte-for-byte.
The three initially dirty ticket files (`check.rs`, `constants.rs`, and `sync.rs`)
are relocated with their working-tree behavior intact. A full-tree whitespace
check also reports an existing trailing blank line in `apps/mod/tbd-export/README.md`;
that unrelated file remains unchanged.
