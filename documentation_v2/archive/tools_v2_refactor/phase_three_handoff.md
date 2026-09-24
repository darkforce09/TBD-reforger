**Status:** archived

# Consolidation record: the ticket subsystem lands in `ticket-engine`

What landed, and the measurements taken at the landing.

## Ownership and interfaces

`ticket-engine` owns validation, command services, generated views, registry readers, wave
scheduling and history, receipts and estimates. Its manifest carries no workspace dependency. The
public ticket model, `Corpus`, `ops`, encoding and vocabulary exports are what `ticketboard`
consumes.

- Validation keeps its schema and rule order, its strict checks, its diagnostics and its refusal
  behaviour. Live writes go through typed operations and surgical corpus persistence.
- The registry readers accept the spellings older blobs carry and restrict the writer accordingly.
  Historical status lookup, cached shipping status and permissive display-title lookup live in the
  same crate.
- Synchronisation owns all six Markdown views, the queue JSON, the roadmap markers and the
  gap-analysis column.
- Ticket CLI services receive the active root. Batch selection invokes a supplied execution
  callback; cleanup target resolution returns the path and branch without deleting either.
- `xtask::commands::ticket` owns agent invocation and worktree and branch cleanup. `ticket done`
  cleans up before shipping. The slice runner writes receipts through `ticket-engine`.
- `xtask::commands::wave` delegates lock and collision commands. The platform wave driver stays in
  `xtask` and shares `ticket-engine`'s close-marker rules: HEAD inclusion, disavowed markers,
  highest against newest claims, shallow-history refusal and archived plan decoding.

Production files in `ticket-engine` are at most 379 lines; test files at most 757. Unit tests live
in sibling test directories. Thirteen size exemptions are removed and none added. Every existing
test function remains; nine tests cover execution, cleanup and the dependency boundaries.

The three execution-receipt fixtures live in `ticket-engine/tests/fixtures/execution_receipts`.
Both crates resolve them from the active checkout and their bytes match the originals. External
package versions, sources and checksums are unchanged.

## Measurements

```text
cargo test --locked -p ticket-engine -p xtask -p ticketboard --no-fail-fast -- --test-threads=1
ticket-engine unit tests:        212 passed
ticket-engine compile-fail:        1 passed
xtask:                           601 passed
ticketboard:                     170 passed, 3 ignored
```

135 relocated tests and nine additions explain the totals against the pre-consolidation counts
(72 / 1 / 732 / 170). No test is removed, ignored or weakened.

| Check | Result |
|---|---|
| `cargo check --locked -p ticket-engine -p xtask -p ticketboard --all-targets` | passes |
| `cargo clippy --locked -p ticket-engine --all-targets -- -D warnings` | passes |
| `cargo fmt --all --check` | passes |
| `cargo doc --locked -p ticket-engine --no-deps` | passes |
| Normal and strict ticket checks, wave checks, citation checks, file-length checks | pass; strict checking also passes against an isolated Git index carrying every new file |
| CLI help and nested-directory invocation, including the Cargo alias | pass |
| Dependency assertions | reject a workspace dependency in `ticket-engine` and a duplicate ticket implementation in `xtask` |

A retained executable supplies the behaviour oracle. Comparisons use identical inputs and the same
executable name for the Clap help:

- Normal and strict ticket checks and wave checks match exit code, stdout and stderr.
- A successful `wave repack` and a successful status change match output and all file bytes on a
  disposable copy; removing an unknown ticket refuses without changing files.
- Help, ticket queries, unknown ids, invalid status changes, metrics, collision refusals and
  dry-run execution match.
- Malformed schemas and tickets, a missing vocabulary or lock, and an invalid lock version
  reproduce the same nonzero exits and diagnostics.
- Synchronisation on a disposable live-corpus copy matches generated file bytes, changed-file sets
  and command output, and a second run is idempotent. The only generated file that differs from
  that copy's initial contents is the ticket registry view, and both executables produce the same
  change. The live generated views are untouched.
- The deterministic fixture tests keep their mutation and refusal, child shipping, SHA stamping,
  no-repack, dependency packing, reservation, empty-wave, history, timestamp, receipt and estimate
  coverage.
