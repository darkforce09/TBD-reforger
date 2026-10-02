# Browser platform source

The source of `browser_platform`: the console macros, the fetch helpers and the crate root that
gates them on wasm32.

## Contents

```text
crates/foundation/browser_platform/src/
├── console.rs  `console_log!`, `console_warn!`, `console_error!` and the functions they call
├── fetch.rs    whole-body, streamed (with `ByteProgress`) and Range GETs over `gloo-net`
├── lib.rs      the crate root: module header, the wasm32 gate and the `mod` lines
└── prelude.rs  the macros and the fetch items for glob import
```

## How it works

`lib.rs` opens with `#![cfg(target_arch = "wasm32")]`, so a native build compiles an empty crate.
The macros are `#[macro_export]`, so they live at the crate root and expand to calls of
`$crate::console`; a caller needs no `web-sys` of its own.

## Boundaries

- Depends on: the browser bindings and `gloo-net`.
- Used by: the map engine, through `fetch` and the crate-root macros.
- Rules: no item knows a caller's vocabulary; progress leaves the crate as byte counts only.
