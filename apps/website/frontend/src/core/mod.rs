//! T-934.1 — core framework utilities: auth store, HTTP client, wire DTOs, SSE,
//! toasts, base UI components, URL scheme guard. All modules ungated (native
//! `cargo test` compiles them; wasm-only bodies are cfg-gated inside the files).

pub mod auth;
pub mod client;
pub mod datefmt;
pub mod dto;
pub mod split_pane;
// T-159.25 SSE consumer (useServerTelemetry port). Transport body is wasm32-gated inside the
// module; the module itself stays ungated so native `cargo test` can run the T-287 Class-R
// abort/teardown source guard (`include_str!("sse.rs")` — same reason dto.rs owns the decode).
pub mod sse;
// T-159.25 — sonner replacement: Toasts context + top-right viewport (renders no DOM while empty).
pub mod toast;
pub mod ui;
// T-405 — the output-side `<a href>` scheme guard. Ungated: pure Rust, no web-sys, so the native
// shell compiles it and `cargo test -p website-frontend` runs its conformance tests against the
// shared case table — which is the thing that stops it drifting from the API's copy.
pub mod url_guard;
