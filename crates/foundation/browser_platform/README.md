# Browser platform

The `browser_platform` crate: the browser console macros and the same-origin HTTP GET helpers
that wasm32 code shares. It compiles only for `wasm32`; on a native build the crate is empty.

## Contents

```text
crates/foundation/browser_platform/
├── Cargo.toml  the package: the browser bindings and `gloo-net` in the wasm32 table, layout tier 0
└── src/        the console macros, the fetch helpers and the prelude
```

## How it works

`console_log!`, `console_warn!` and `console_error!` format their arguments like `format!` and
write one line to the browser console at their level, through `console::log`, `console::warn`
and `console::error`.

`fetch::fetch_bytes` and `fetch::fetch_text` read the whole body of a GET and answer `None` on a
transport failure or a status outside 2xx. `fetch::open_streamed_body` opens a 2xx body and
reports its `content-length`; `StreamedBody::read_to_end` hands each chunk the browser's body
reader yields to a sink and reports `ByteProgress` (bytes received, announced length): once with
zero bytes before the first chunk, then each time at least the caller's granularity more arrived,
then once for the remainder. `fetch::fetch_bytes_streamed` collects that body into one buffer.
The caller turns `ByteProgress` into its own vocabulary; the map engine's streaming host makes it
the boot bar's byte budget and bytes done. `fetch::fetch_range_outcome` sends a Range GET and
answers a body only for a 206 with a `Content-Range` total, a rate limit with its `Retry-After`
for a 429, and a failure with the status for anything else, a 200 included, so a server that
ignores `Range` never sends a whole file.

## Getting started

Run from the repository root:

```bash
cargo clippy -p browser_platform --target wasm32-unknown-unknown -- -D warnings
```

The crate holds no native test: every function calls a browser API.

## Configuration

No feature and no environment variable.

## Public surface

- `console_log!`, `console_warn!`, `console_error!` (crate root) and `console::{log, warn, error}`.
- `fetch`: `fetch_bytes`, `fetch_text`, `open_streamed_body`, `StreamedBody`
  (`content_length`, `read_to_end`), `fetch_bytes_streamed`, `ByteProgress`,
  `fetch_range_outcome`, `RangeBody`, `RangeOutcome`.
- `prelude`: the macros and the fetch items.

## Boundaries

- Depends on: `web-sys` (`console`, `ReadableStream`, `ReadableStreamDefaultReader`), `js-sys`,
  `wasm-bindgen`, `wasm-bindgen-futures` and `gloo-net`, all on wasm32 only.
- Used by: the map engine's `render` tier (`legacy/map_engine`): its satellite imagery, streaming
  host and occluder loader log through the console macros, and its streaming host, loaders and DEM
  loader fetch through `fetch`; the single-page app's map view mount and world line-of-sight bench
  (`apps/frontend`) fetch through `fetch` too.
- Rules: foundation tier, so the crate depends on no workspace crate
  (`cargo xtask verify crate-tiers`); `targets = "wasm32"`, so `lib.rs` gates every item on
  `target_arch = "wasm32"` and a native crate reaches it only from its wasm32 dependency table.

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the dependency
  directions between the workspace crates.
