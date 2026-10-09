# Time source

The `time_source` crate: the workspace's one wall-clock trait with its real and test clocks, a
monotonic millisecond source for frame timing and work budgets, and UTC timestamps in RFC 3339,
written to the millisecond or the second and validated against the canonical-UTC rule.

## Contents

```text
crates/foundation/time_source/
├── Cargo.toml  the package: `thiserror`, `time` (parsing), `js-sys` and `web-sys` on wasm32 only
└── src/        the clocks, the monotonic source, the formatters, the validation and their tests
```

## How it works

**Clocks.** `Clock` answers one question, `now_unix_ms() -> u64`: milliseconds since
1970-01-01T00:00:00Z, 0 before it, with `now_unix_ms_f64()` for callers that work in the
`f64` milliseconds of `Date.now()`. `SystemClock` reads `std::time::SystemTime` natively;
`BrowserClock` reads `Date.now()` on `wasm32`, where `SystemTime::now` panics; `PlatformClock`
names whichever of the two the target has, as a value (`PlatformClock.now_unix_ms()`), and
`wall_clock_ms() -> f64` is its `f64` reading as a plain function: `Date.now()` itself on
`wasm32`, so a call site that read `js_sys::Date::now()` reads the same value through it.
`ManualClock` stands still until a test sets or advances it, through `&self`, so one
`Arc<ManualClock>` drives the code under test. The trait is `Send + Sync`, so an
`Arc<dyn Clock>` crosses threads and can back a clock trait of another library through a small
adapter.

**Monotonic time.** `monotonic_ms() -> f64` measures durations: `performance.now()` on `wasm32`
(`Date.now()` where the global scope has no `window`), `std::time::Instant` from a process-wide
origin natively. It is a plain `fn() -> f64`, so it fills a function-pointer clock slot as is.

**Timestamps.** `rfc3339_utc_millis(unix_ms)` and `iso_from_system_time(time)` write
`2026-07-04T23:43:38.437Z`, as JavaScript's `toISOString()` does; `rfc3339_utc_seconds(unix_s)`
and `now_utc_rfc3339()` write `2026-08-14T12:34:56Z`. Both use Howard Hinnant's civil-from-days
algorithm and truncate. `validate_rfc3339_utc(field, value)` accepts a value that parses as
RFC 3339 (through the `time` crate), carries a zero offset written `Z` or `+00:00`, and separates
date and time with an uppercase `T`; each refusal is an `Error` that names the field and the
value. Everything the formatters write passes it.

| Replaced code | Item here | Behaviour to know when adopting |
|---|---|---|
| map renderer frame timing, render diagnostics benchmark clocks | `monotonic_ms`, `BrowserClock.now_unix_ms_f64()` | `performance.now()` with the `Date.now()` fallback; the wall clock is `Date.now()` |
| map engine viewshed scheduler host clock | `monotonic_ms` in `SchedulerHost::now_ms` | the wasm fallback advances with real time instead of one tick per call |
| the inline `js_sys::Date::now()` readings of the streaming host, the asset loader and the frontend crates | `wall_clock_ms` | identical values on `wasm32` |
| the frontend's `new Date().toISOString()` readings | `rfc3339_utc_millis(PlatformClock.now_unix_ms())` | identical text for years 0 to 9999 |
| the event manager's `new Date()` for today's local calendar day | a `js_sys::Date` built from `wall_clock_ms()` | identical local calendar day |
| the validation panel's `performance.now()` debounce clock | `monotonic_ms` | identical in a window; a worker falls back to `Date.now()` instead of `0` |
| the API metrics registry's `SystemTime` start stamp | `PlatformClock.now_unix_ms() / 1000` | identical whole seconds |
| map engine CRDT undo-group `RealClock`, `ManualClock`, injected wasm clock | `PlatformClock`, `ManualClock`, `BrowserClock` | the `yrs::sync::Clock` adapter keeps the old floor of 1 ms |
| developer tools `iso_from_system_time` | `iso_from_system_time` | byte-identical |
| ticket tools `now_utc_rfc3339`, `validate_rfc3339_utc` | the same names | byte-identical output; `Error` replaces `String`, with the same text |

## Getting started

Run from the repository root:

```bash
cargo test -p time_source   # the clocks, the formatters against `time`, the validation cases
```

## Configuration

No features and no environment variables. The browser crates are dependencies on `wasm32` only.

## Public surface

- `Clock` (`now_unix_ms`, `now_unix_ms_f64`); `SystemClock` (not on `wasm32`); `BrowserClock`
  (`wasm32` only); `PlatformClock`; `wall_clock_ms() -> f64`; `ManualClock` (`new`, `set`,
  `advance`).
- `monotonic_ms() -> f64`.
- `rfc3339_utc_millis(u64)`, `iso_from_system_time(SystemTime)`, `rfc3339_utc_seconds(u64)`,
  `now_utc_rfc3339()`, each `-> String`.
- `validate_rfc3339_utc(field, value) -> Result<()>`; `Error`, `Result`.
- `prelude`: everything above except `Error` and `Result`.

## Boundaries

- Depends on: `thiserror`, `time` (parsing); on `wasm32`, `js-sys` and `web-sys` (`Window`,
  `Performance`).
- Used by: the map renderer, the render diagnostics, the map editing tools, the mission CRDT and
  document crates, the streaming host and asset loader, the Mission Creator workspace crates
  (engine bridge, session, workspace, debug benches), the administration pages, `frontend_ui`,
  the API HTTP layer, and the developer, ticket, staging and map asset tools. Every
  `Date.now()`, `performance.now()` and `SystemTime::now()` reading of the library crates comes
  through it.
- Rules: the ticket tools' accept and reject cases hold (`accepts_canonical_utc`,
  `rejects_malformed_and_non_utc`); the whole-second form matches the `time` crate's RFC 3339
  output (`the_whole_second_form_matches_the_time_crate`); the crate firewall admits browser
  crates here only from the `wasm32` target table (`cargo xtask verify crate-tiers`).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
