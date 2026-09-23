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

pub mod auth;
pub mod common;
pub mod content;
pub mod event_access_administration;
pub mod event_viewer_access;
pub mod events;
pub mod fleet_commands;
pub mod fleet_scenarios;
pub mod mission_deployments;
pub mod mission_reviews;
pub mod missions;
pub mod registry;
pub mod servers;
pub mod telemetry;

pub use auth::*;
pub use common::*;
pub use content::*;
pub use event_access_administration::*;
pub use event_viewer_access::*;
pub use events::*;
pub use fleet_commands::*;
pub use fleet_scenarios::*;
pub use mission_deployments::*;
pub use mission_reviews::*;
pub use missions::*;
pub use registry::*;
pub use servers::*;
pub use telemetry::*;

#[cfg(test)]
#[path = "tests/shapes.rs"]
mod shapes;

#[cfg(test)]
#[path = "tests/r_api.rs"]
pub(crate) mod r_api;
