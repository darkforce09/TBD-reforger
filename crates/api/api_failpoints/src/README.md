# API failpoints source

The source of `api_failpoints`: the `fail_point!` macro and, behind the `failpoints` feature, the
catalogue of named points, the process-global registry that arms them, the pause handle and the
failure an armed point returns.

## Contents

```text
crates/api/api_failpoints/src/
├── catalogue.rs         `Failpoint`, one variant per named point, and `CATALOGUE`, every entry
├── error.rs             `Error::InjectedFailure` and its conversions into `ApiError` and `sqlx::Error`
├── fail_point_macro.rs  the `fail_point!` macro, off and on
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── pause_handle.rs      `PauseHandle`: learn that a request arrived at a point, then let it go on
├── prelude.rs           `fail_point!` for glob import
├── registry.rs          the armed entries, the suite lock, `arm`, `ArmGuard` and `reach`
└── tests/               unit tests for the catalogue, the pause handle, the registry and the macro
```

## How it works

`lib.rs` compiles every module but the macro only with the `failpoints` feature, so a build
without it holds the macro alone, which expands to nothing. The macro expands through `$crate::`,
so it resolves `reach` and `Failpoint` in this crate from any call site. `catalogue.rs` adds two
points that exist only in this crate's unit-test build, which the macro and registry tests arm.

```text
test case ─▶ lock_suite ─▶ arm(point, action) ─▶ ArmGuard ─┐
request ─▶ call site fail_point!(point) ─▶ reach(point) ───┴▶ pass / fail / hold until release
```

## Boundaries

- Depends on: `api_foundation::error_handling::api_error::ApiError` and `sqlx::Error` for the two
  conversions; `tokio::sync` for the suite lock and the pause flags.
- Used by: the crate root and, through it, every call site.
- Rules: a new point is a new `Failpoint` variant listed in `CATALOGUE` and in the catalogue unit
  test.
