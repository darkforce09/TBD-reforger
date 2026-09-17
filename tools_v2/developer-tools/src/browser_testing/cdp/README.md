# CDP Client & Process Spawner (`browser_testing/cdp`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Decomposed from the 704-line `cdp.rs` file into focused submodules (<400 LOC each).

---

## Responsibilities

- **`process.rs`**: Discovers the pinned Chromium binary, sets SwiftShader WebGL2/WebGPU GPU flags, and spawns the browser with an isolated temp user data directory.
- **`client.rs`**: Manages the `tokio-tungstenite` WebSocket session, dispatches CDP commands (`Page.navigate`, `Runtime.evaluate`, `Input.dispatchMouseEvent`, `Input.dispatchKeyEvent`), and intercepts browser panic logs.
