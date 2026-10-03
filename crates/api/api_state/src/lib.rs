//! The API's application state: the one dependency container of the HTTP layer.
//!
//! **Role:** holds [`AppState`] and its `FromRef` projections, so a handler, an extractor or a
//! middleware takes the whole state or exactly the part it needs.
//! **Position:** above `api_http_layer`, `api_configuration`, `api_discord` and
//! `api_equipment_datasets`; the API's composition root builds it with the concrete services, and
//! every domain, background worker, the router and the integration suites receive it; names no
//! domain and no other caller identity than the `SessionAuthority` trait object.
//! **Signals & state:** see [`application_state`]: every field is an `Arc` or a pool handle.
//! **Invariants:** the state never builds a service that needs a concrete implementation; the
//! session authority, the Discord and webhook clients and the equipment datasets are injected.

pub mod application_state;
pub mod prelude;

pub use application_state::AppState;
