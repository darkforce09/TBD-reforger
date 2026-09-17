# Headless Browser Testing Infrastructure (`developer-tools/src/browser_testing`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Automated end-to-end regression testing framework for the Leptos CSR single-page application using Chrome DevTools Protocol (CDP).

---

## Submodules

- **`cdp/`**: Low-level Chromium launcher with WebGL2 SwiftShader software flags and async WebSocket client.
- **`server/`**: Embedded static SPA file server and Axum reverse proxy matching production routing.
- **`diagnostics/`** (formerly `doctor.rs`): Preflight checks ensuring fonts, memory, and Chrome binary pins match CI.
- **`dom_oracle/`** (formerly `vsuite.rs`): Golden DOM tree snapshot comparator for visual regression detection.
- **`route_drift/`** (formerly `sroutes.rs`): Static AST route analyzer detecting unexercised frontend routes.
- **`editor_smoke_tests/`**: Modularized end-to-end test scenarios driving the 2D/3D Mission Creator CAD workspace.
