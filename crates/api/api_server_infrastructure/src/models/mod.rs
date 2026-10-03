//! Database and wire models for the dedicated-server fleet.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`fleet_wire_contract::rfc3339_timestamps`].

pub mod fleet_command;
pub mod fleet_scenario;
pub mod machine_credential;
pub mod server;
