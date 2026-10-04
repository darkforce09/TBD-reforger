//! The single-page app's wire types: one Rust shape per JSON body the API sends or accepts.
//!
//! **Role:** groups the data transfer objects by the domain they belong to, re-exports most of
//! them flat so a caller names the type rather than the file it lives in, and declares the typed
//! identifiers every one of them holds ([`identifiers`]).
//! **Position:** the boundary between the HTTP transport and everything that renders. Nothing here
//! fetches, stores or decides anything; the transport, the session, the pages and the Mission
//! Creator name these types.
//! **Signals & state:** none — every type in this crate is plain data.
//! **Invariants:** field names are the API contract. The backend's models are the source of truth,
//! and these mirror them; where the two disagree the backend wins. Every type is round-tripped
//! against a captured response, so an added field that the fixtures do not carry fails the tests
//! rather than passing silently. The mission and ballistics types a DTO carries are named from
//! their own crates, never re-exported here.

pub mod administration;
pub mod auth;
pub mod ballistics_catalogs;
pub mod common;
pub mod content;
pub mod event_access_administration;
pub mod event_viewer_access;
pub mod events;
pub mod fire_missions;
pub mod fleet_commands;
pub mod fleet_scenarios;
pub mod identifiers;
/// A page of a match's detailed events: test-only, since no page reads them; the mirror exists for
/// its round-trip tests.
#[cfg(test)]
pub mod match_events;
pub mod mission_deployments;
pub mod mission_reviews;
pub mod missions;
pub mod prelude;
pub mod registry;
pub mod role;
pub mod servers;
pub mod telemetry;
pub mod vehicles;
pub mod wiki;

pub use auth::*;
pub use common::*;
pub use content::*;
pub use event_access_administration::*;
pub use event_viewer_access::*;
pub use events::*;
pub use fire_missions::*;
pub use fleet_commands::*;
pub use fleet_scenarios::*;
#[cfg(test)]
pub use match_events::*;
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

pub mod equipment_data_viewer;

#[cfg(test)]
#[path = "tests/equipment_data_viewer_parity.rs"]
mod equipment_data_viewer_parity;
