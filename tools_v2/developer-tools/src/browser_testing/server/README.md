# Embedded Test Web Server (`browser_testing/server`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Decomposed from the 760-line `serve.rs` file into focused submodules (<400 LOC each).

---

## Responsibilities

- **`static_server.rs`**: Fast static file server hosting the Leptos WASM bundle from `apps/website/frontend/dist` with proper MIME mappings.
- **`api_proxy.rs`**: Reverse proxy forwarding `/api/*` and `/map-assets/*` requests to the mock API server, injecting test authentication cookies.
