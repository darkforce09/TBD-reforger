//! Server control: the configured game servers, their state, and the console that commands them.
//!
//! **Role:** declares the route component, the picker and server card, the RCON console, and the
//! channel all three send through.
//! **Position:** the `/admin/server` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and every shared signal.
//! **Invariants:** every control on this screen reaches the host through one send path, so there is
//! exactly one place that decides whether a reply reports success.
#![allow(dead_code)]

mod page;
mod rcon;
mod rcon_console;
mod server_cards;

pub use page::ServerControlPage;

#[cfg(test)]
use crate::v2::core::api::dto::ServerRowDto;
#[cfg(test)]
use rcon::{
    admin_server_rcon_path, classify_prompt_field, rcon_accepted_message, rcon_body_change_map,
    rcon_body_custom, rcon_body_kick, rcon_body_restart, rcon_reports_success, PromptField,
    RconAccepted,
};
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use server_cards::pick_default_id;

#[cfg(test)]
#[path = "tests/server_control.rs"]
mod t270;
