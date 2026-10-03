//! The wire types: one Rust shape per JSON body the backend sends or accepts.
//!
//! **Role:** groups the data transfer objects by the domain they belong to, and re-exports them
//! flat so a caller names the type rather than the file it lives in.
//! **Position:** the boundary between the HTTP client and everything that renders. Nothing here
//! fetches, stores or decides anything.
//! **Signals & state:** none — every type in this tree is plain data.
//! **Invariants:** field names are the API contract. The backend's models are the source of truth,
//! and these mirror them; where the two disagree the backend wins. Every type is round-tripped
//! against a captured response, so an added field that the fixtures do not carry fails the tests
//! rather than passing silently.

#[cfg(any(target_arch = "wasm32", test))]
pub mod administration;
pub mod auth;
pub mod ballistics_catalogs;
#[cfg(any(target_arch = "wasm32", test))]
pub mod common;
pub mod content;
pub mod event_access_administration;
pub mod event_viewer_access;
pub mod events;
pub mod fire_missions;
pub mod fleet_commands;
pub mod fleet_scenarios;
// Test-only: no page reads match events; the mirror exists for its round-trip tests.
#[cfg(test)]
pub mod match_events;
pub mod mission_deployments;
#[cfg(any(target_arch = "wasm32", test))]
pub mod mission_reviews;
#[cfg(any(target_arch = "wasm32", test))]
pub mod missions;
#[cfg(any(target_arch = "wasm32", test))]
pub mod registry;
pub mod role;
pub mod servers;
pub mod telemetry;
pub mod vehicles;
#[cfg(any(target_arch = "wasm32", test))]
pub mod wiki;

pub use auth::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use common::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use content::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use event_access_administration::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use event_viewer_access::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use events::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use fire_missions::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use fleet_commands::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use fleet_scenarios::*;
#[cfg(test)]
pub use match_events::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use mission_deployments::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use mission_reviews::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use missions::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use registry::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use servers::*;
#[cfg(any(target_arch = "wasm32", test))]
pub use telemetry::*;

#[cfg(test)]
#[path = "tests/shapes.rs"]
mod shapes;

#[cfg(test)]
#[path = "tests/r_api.rs"]
pub(crate) mod r_api;

pub mod equipment_data_viewer;

#[cfg(test)]
#[path = "tests/equipment_data_viewer_parity.rs"]
mod equipment_data_viewer_parity;
