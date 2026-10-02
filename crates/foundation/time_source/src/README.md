# Time source source

The source of `time_source`: the clock trait and its clocks, the monotonic source, the UTC
formatters, the canonical-UTC validation, and the crate root that exports them.

## Contents

```text
crates/foundation/time_source/src/
├── browser_clock.rs   `BrowserClock`: `Date.now()`, compiled on wasm32 only
├── clock.rs           `Clock`, the wall-clock trait, and `PlatformClock`, the target's real clock
├── error.rs           `Error` and `Result`: why a timestamp fails the canonical-UTC rule
├── lib.rs             the crate root: module header, `mod` lines and re-exports
├── manual_clock.rs    `ManualClock`: the test clock, set and advanced by hand
├── monotonic.rs       `monotonic_ms`: `performance.now()` on wasm32, `Instant` natively
├── prelude.rs         the clocks, the monotonic source, the formatters and the validation
├── system_clock.rs    `SystemClock`: `SystemTime`, compiled off wasm32 only
├── tests/             unit tests of the clocks, the formatters and the validation
├── utc_format.rs      the millisecond and whole-second RFC 3339 UTC formatters
└── utc_validation.rs  `validate_rfc3339_utc`: the canonical-UTC rule
```

## How it works

`lib.rs` re-exports every public item at the crate root; `prelude.rs` re-exports the same set
except the error type. `browser_clock.rs` and `system_clock.rs` each gate their whole module on
the target, and `clock.rs` names the one present as `PlatformClock`. `utc_format.rs` turns
seconds since the epoch into a calendar date and time with Howard Hinnant's civil-from-days
algorithm and writes them with or without milliseconds; `utc_validation.rs` parses with the
`time` crate and applies the zero-offset, `Z`-or-`+00:00` and uppercase-`T` checks in that
order.

## Boundaries

- Depends on: `thiserror`, `time`; `js-sys` and `web-sys` on wasm32.
- Used by: callers through the crate root or `prelude`.
- Rules: no file outside `browser_clock.rs` and `monotonic.rs` names a browser crate, and both
  gate that code on `target_arch = "wasm32"`.
