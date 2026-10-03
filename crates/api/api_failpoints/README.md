# API failpoints

The `api_failpoints` crate: fault injection for the test builds of the
[API](/documentation/glossary/a_to_f.md#api). It holds named points on the commit and
external-effect paths where a test makes the path fail or pause, so the failure and race suites
reach, on demand, the states a live request reaches only by bad luck: a transaction that dies
before its commit, a response lost after it, two requests interleaved at an exact line. A deploy
build compiles all of it out.

## Contents

```text
crates/api/api_failpoints/
├── Cargo.toml  the package: the dev-only `failpoints` feature, `api_foundation`, layout tier 2
└── src/        the macro, the catalogue, the registry, the pause handle and the injected failure
```

## How it works

The Cargo feature `failpoints` is not a default feature. Only `[dev-dependencies]` edges turn it
on: this crate's dev-dependency on itself, the dev-dependency of every API crate that places a
call site (`api_administration`, `api_identity_and_access`, `api_match_telemetry`,
`api_missions`, `api_operations`, `api_server_infrastructure`) and the API application's
dev-dependency on it. So every test build (this crate's unit tests, each of those crates' unit
tests, and every `apps/api/tests/*.rs` binary) carries it, and a build of the `api` binary
(`cargo build --release -p api --bin api`, the deploy build) does not.

A call site names a point with `fail_point!(<variant>);` inside an `async fn` whose error type
converts from `api_failpoints::Error`, as `ApiError` and `sqlx::Error` do. Without the
feature the macro expands to nothing. With it, the call site asks the registry what to do:

| Armed action | First arrival | Later arrivals |
|---|---|---|
| none | passes | passes |
| `FailAction::Fail` | returns the injected failure | return the injected failure |
| `FailAction::FailOnce` | returns the injected failure | pass |
| `FailAction::Pause(handle)` | waits until `handle.release()` | pass |

A returned failure leaves the enclosing function at the point, so an open transaction drops and
rolls back; after a commit, the writes stand and the caller gets `500 internal error`, the lost
response. `ApiError` carries `details.failpoint` naming the point; `sqlx::Error` is a `Protocol`
error whose text names it.

The registry is one per process, so the cases of a test binary, which run in parallel threads,
take turns: a case calls `lock_suite().await`, arms through the lock it holds
(`suite.arm(Failpoint::X, action)`), and keeps the returned `ArmGuard` alive for as long as the
point stays armed. The guard borrows the lock, so it cannot outlive it; dropping it disarms the
point and releases a pause it still holds, so a held request never outlives its case. A pause's
`reached()` panics when no request arrives within `PAUSE_REACH_BOUND`, failing the case instead
of hanging the binary; `ArmGuard::arrivals` counts the requests that reached the point.

```text
test case ─▶ lock_suite ─▶ arm(point, action) ─▶ ArmGuard ─┐
request ─▶ call site fail_point!(point) ─▶ reach(point) ───┴▶ pass / fail / hold until release
```

## Getting started

Run from the repository root:

```bash
cargo test -p api_failpoints   # the catalogue, pause handle, registry and macro unit tests
```

To add a point, add a `Failpoint` variant, list it in `CATALOGUE` and in the catalogue unit test,
and place `fail_point!(<Variant>);` at the call site.

## Configuration

One feature, `failpoints`, off by default and enabled only from `[dev-dependencies]`. The crate
reads no environment variable.

## Public surface

- `fail_point!`, at the crate root and in `prelude`; with the feature off it expands to nothing.
- With `failpoints`: `Failpoint` and `CATALOGUE`; `Error` (`Error::InjectedFailure`) and
  `Result`, with `From<Error>` for `ApiError` and for `sqlx::Error`; `reach`, `lock_suite`,
  `FailpointSuiteLock`, `ArmGuard` and `FailAction`; `PauseHandle` and `PAUSE_REACH_BOUND`.

## Boundaries

- Depends on: `api_foundation` (`ApiError`, which the injected failure converts into), `axum`,
  `serde_json`, `sqlx`, `thiserror`, `tokio` (the suite lock and the pause flags) and `tracing`.
- Used by: the call sites in the services and handlers of the six API domain crates above; the
  `failure_injection*` and
  `controlled_races*` suites under `apps/api/tests/`, the only binaries that arm catalogue
  points; this crate's unit tests, which arm only the two points that exist in its unit-test
  build.
- Rules: never enable `failpoints` by default or on a deploy build, which
  `apps/api/tests/engineering_laws.rs` and `cargo xtask ci verify-workspace-laws` check; a call
  site passes a catalogue point; every arming case holds the suite lock for its whole run.

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [Website API](/apps/api/README.md) — the application whose test builds arm the points.
