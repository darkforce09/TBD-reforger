# Failpoints

Fault injection for the test builds of the [API](/documentation_v2/glossary/a_to_f.md#api):
named points on the commit and external-effect paths where a test makes the path fail or pause,
so the failure and race suites reach, on demand, the states a live request reaches only by bad
luck: a transaction that dies before its commit, a response lost after it, two requests
interleaved at an exact line. A deploy build compiles all of it out.

## Contents

```text
apps/website/api_v2/src/core/failpoints/
├── catalogue.rs         `Failpoint`, one variant per named point, and `CATALOGUE`, every entry
├── injected_failure.rs  `InjectedFailure` and its conversions into `ApiError` and `sqlx::Error`
├── mod.rs               the `fail_point!` macro, off and on, and the module's public surface
├── pause_handle.rs      `PauseHandle`: learn that a request arrived at a point, then let it go on
├── registry.rs          the armed entries, the suite lock, `arm`, `ArmGuard` and `reach`
└── tests/               unit tests for the catalogue, the pause handle, the registry and the macro
```

## How it works

The Cargo feature `failpoints` is not a default feature. The crate turns it on for itself through
a dev-dependency on its own path, so every test build (the library's unit tests and every
`apps/website/api_v2/tests/*.rs` binary) carries it, and a build of the `api` binary
(`cargo build --release -p website-api --bin api`, the deploy build) does not.

A call site names a point with `fail_point!(<variant>);` inside an `async fn` whose error type
converts from `InjectedFailure`, as `ApiError`, `sqlx::Error` and `anyhow::Error` do. Without the
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

## Boundaries

- Depends on: `tokio::sync` (the suite lock and the pause flags), `serde_json`, `tracing`,
  `crate::core::error_handling::api_error::ApiError` and `sqlx::Error` for the two conversions.
- Used by: the call sites in the domains' services and handlers; the `failure_injection*` and
  `controlled_races*` suites under `apps/website/api_v2/tests/`, the only binaries that arm
  catalogue points; this module's unit tests, which arm only the two points that exist in the
  library's unit-test build.
- Rules: never enable `failpoints` by default or on a deploy build; a call site passes a
  catalogue point, and a new point is a new `Failpoint` variant listed in `CATALOGUE` and in the
  catalogue unit test; every arming case holds the suite lock for its whole run.
