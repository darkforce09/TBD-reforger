**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Frontend transport layout specification

**Status:** Proposed design  
**Scope:** `<apps/website/frontend/src/v2/core/transport/>`  
**Context:** TBD Reforger platform monorepo  

Detailed architectural specification for the frontend network transport layer, replacing the
misnamed `core/api/` module with a dedicated, browser-aware transport engine and eliminating
duplicated wire DTO definitions.

---

## 1. Architectural Role & Why "Transport"

Calling a frontend directory `api/` creates fundamental confusion in a full-stack monorepo:
* Developers looking to update an API endpoint or response format often look in the frontend's
  `core/api/` instead of `apps/website/api_v2/`.
* It misrepresents what the frontend code does: the frontend **does not host or define an API**; it
  **transports data** across the network between the browser DOM and the remote Axum server.

Renaming this subsystem to **`transport`** establishes a clear systems-level mental model:
* **`apps/website/api_v2/`** = **THE API** (server routes, SQL queries, business rules, and shared wire types).
* **`frontend/.../core/transport/`** = **THE TRANSPORT ENGINE** (browser HTTP verbs, tab concurrency locks,
  token refresh transactions, and realtime SSE streams).

---

## 2. Proposed Directory Structure

```text
apps/website/frontend/src/v2/core/transport/
├── README.md                   # Transport architecture, error handling, & concurrency contracts
├── mod.rs                      # Module tree & public surface re-exports
├── client/                     # Low-level browser HTTP verbs & retry policies
│   ├── mod.rs
│   ├── errors.rs               # ApiErr failure representations & error body parsing
│   ├── fetched.rs              # Reactive signal container for asynchronous requests
│   ├── public_reads.rs         # Unauthenticated fetch helpers
│   ├── refresh.rs              # Single-flight 401 refresh, Web Locks, & BroadcastChannel
│   ├── refusals.rs             # ApiRefusal structured domain rejection reasons
│   └── requests.rs             # [wasm32 only] api_get, api_post, api_put, api_delete, api_upload_file
├── actions/                    # Typed frontend action callers (formerly endpoints/)
│   ├── mod.rs                  # Path segment encoding & JSON body builders
│   ├── event_access.rs         # Policy changes & admin grants
│   ├── event_registration.rs   # Mission registration & waitlist promotion
│   ├── fleet_commands.rs       # Server command submission & cancellation
│   ├── fleet_scenarios.rs      # Scenario listing & deployment
│   ├── machine_credentials.rs  # Credential issuance & revocation
│   ├── mission_deployments.rs  # Deployment triggering & status queries
│   └── mission_reviews.rs      # Community review submissions & verdicts
├── sse.rs                      # Live server status ReadableStream telemetry consumer
└── tests/                      # Native unit tests for refresh logic & stream teardown
```

---

## 3. Subsystem Breakdown

### 3.1. Browser HTTP Verbs (`client/`)
* **Role**: The foundational HTTP engine that every page and CAD workspace uses to talk to the backend.
* **Invariants**:
  - Pure browser implementation (`#[cfg(target_arch = "wasm32")]`) built on `gloo-net`, `web-sys`,
    and `wasm-bindgen-futures`.
  - Automatically attaches the Bearer access token from the active session.
  - Implements an invariant single retry: if a request answers `401 Unauthorized`, it pauses behind
    the single-flight refresh transaction, re-authenticates once, and retries with the rotated token.
    If the retry also answers 401, the session is formally terminated and the caller receives the error.
  - Plain verbs return `Result<T, ApiErr>`; refusal-keeping verbs return `Result<T, ApiRefusal>`.

### 3.2. Session Continuity & Concurrency (`client/refresh.rs`)
* **Role**: Ensures that multiple open browser tabs or concurrent background requests do not invalidate
  the user's session when rotating single-use refresh tokens.
* **Mechanisms**:
  1. **Single Flight**: An in-memory cell ensures that 10 concurrent 401s in the same tab trigger
     only **one** refresh request to `POST /api/v1/auth/refresh`. The remaining 9 await its outcome.
  2. **Web Locks API (`navigator.locks`)**: Coordinates across multiple open browser tabs. When tab A
     acquires the refresh lock, tab B waits rather than making a competing call that would fail due to
     token reuse.
  3. **BroadcastChannel**: Announces rotated session tokens to all peer tabs so their memory state
     updates synchronously without reloading.

### 3.3. Realtime Telemetry Stream (`sse.rs`)
* **Role**: Consumes the live server status stream at `GET /api/v1/servers/{id}/status/stream` for the
  Server Intel command console.
* **Why not browser `EventSource`**: Standard browser `EventSource` cannot carry HTTP `Authorization`
  headers. `sse.rs` implements streaming over `fetch()` with a `web_sys::ReadableStreamDefaultReader`.
* **Invariants**:
  - Accumulates byte chunks and splits cleanly on double-newlines (`\n\n`).
  - Decodes frames using `website_api_types::decode_server_status_frame`.
  - Pushes frame updates directly into Leptos reactive signals (`WriteSignal<Option<ServerStatusDto>>`).
  - Stores the active `AbortController` in a thread-local cell, exposing a zero-capture function
    `abort_server_status_stream()` for Leptos route cleanup (`on_cleanup`).

### 3.4. Typed Action Callers (`actions/`)
* **Role**: High-level async functions called directly by UI event handlers (e.g. clicking "Register"
  or "Issue Fleet Command").
* **Signature Pattern**:
  ```rust
  pub async fn register_for_mission(
      store: AuthStore,
      event_mission_id: &str,
      seat: Option<&str>,
  ) -> Result<ReservationResponse, ApiRefusal> {
      let path = mission_registration_path(event_mission_id);
      api_post_keeping_refusal(store, &path, registration_body(seat)).await
  }
  ```
* **Why it remains in Frontend**: These functions require `AuthStore` (Leptos reactive session context)
  and browser-only fetch verbs.

---

## 4. Elimination of Duplicate DTOs

In the current codebase, `frontend/src/v2/core/api/dto/` hand-defines dozens of structs that mirror
contracts already defined in `contracts_v2/` and generated in `api_v2/`.

### What Is Eliminated:
1. **`dto/` folder deleted**: All manual struct definitions (`dto/events.rs`, `dto/servers.rs`,
   `dto/missions.rs`, `dto/equipment_data_viewer/`, etc.) are removed.
2. **`dto/tests/` folder deleted**: 15+ parity test files (`r_api_auth.rs`, `r_api_events.rs`,
   `equipment_data_viewer_parity.rs`) are removed.

### How Types Are Consumed:
Frontend imports wire data shapes directly from the new shared crate:
```rust
use website_api_types::{
    EventListItem, OrbatSlot, ServerStatusDto, EquipmentDatasetStatus,
};
```
Because `website_api_types` contains zero native server runtimes (no Tokio, no SQLx, no Hyper), it
compiles cleanly to `wasm32-unknown-unknown`. When the API adds or changes a field, the frontend
compiler immediately verifies the change at compile time.

---

## 5. Public Surface & Invariants

* **Public exports from `crate::v2::core::transport`**:
  - `client`: `api_get`, `api_post`, `api_put`, `api_patch`, `api_delete`, `api_upload_file`,
    `api_post_ok`, `api_post_raw`, `bootstrap`, `ApiErr`, `ApiRefusal`.
  - `actions`: Typed route callers (`register_for_mission`, `request_fleet_command`, etc.).
  - `sse`: `stream_server_status`, `abort_server_status_stream`.
* **Zero-Churn Alias**: During migration, `pub use transport as api;` in `src/v2/core/mod.rs` prevents
  breaking legacy imports.
