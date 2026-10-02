//! Server control: the configured game servers, their registration, their state, the fleet
//! commands and mission deployments that act on them, the fleet scenario registry, and the machine
//! credentials their programs authenticate with.
//!
//! **Role:** declares the route component, the server registry with its registration sheet, the
//! picker and server card with its telemetry band, the fleet command console, the deployments
//! panel, the fleet scenario sheet, and the machine-credential sheet each server card opens.
//! **Position:** the `/admin/server` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the server registry and the scenario
//! registry, and each server card owns the state of its console, its deployments panel and its
//! credential sheet.
//! **Invariants:** nothing on this screen reaches a host directly. A command or a deployment is a
//! request the backend records and answers with 202; the screen then follows its receipt to the
//! outcome an executor or a runtime session reports, and says nothing happened before it did. A
//! registration, a change or a deactivation changes the server's row and nothing on the host.
#![allow(dead_code)]

mod fleet_commands;
mod fleet_scenarios;
mod machine_credentials;
mod mission_deployments;
mod page;
mod server_card_telemetry;
mod server_cards;
mod server_registry;

pub use page::ServerControlPage;

#[cfg(test)]
use crate::v2::core::api::dto::ServerRowDto;
#[cfg(test)]
use server_card_telemetry::{format_queue_age, queue_fill};
#[cfg(test)]
use server_cards::terrain_reading;
#[cfg(test)]
use server_registry::pick_default_id;

#[cfg(test)]
#[path = "tests/server_control.rs"]
mod tests;
