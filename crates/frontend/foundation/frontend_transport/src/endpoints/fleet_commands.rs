//! The administrator routes of one server's fleet commands: request, follow and cancel.
//!
//! **Role:** the command collection, receipt and cancellation paths, and one call per route.
//! **Position:** called by the server control screen's command console.
//! **Signals & state:** none.
//! **Invariants:** a request answers 202 with the receipt of an accepted command, and nothing about
//! its outcome — the receipt is followed until the command reaches a terminal state. The operator
//! route refuses the two actions only a mission deployment issues. Only a command no executor has
//! claimed can be cancelled; any other state answers `409 COMMAND_NOT_CANCELLABLE` with that state
//! in the details. A kick against a runtime session that is no longer the server's open one answers
//! `409 RUNTIME_SESSION_ENDED`.

use super::encode_path_segment as segment;

/// `GET` (the server's commands, newest first) and `POST` (request one) `/servers/:id/commands`.
pub fn server_commands_path(server_id: &ServerId) -> String {
    format!("/servers/{}/commands", segment(server_id))
}

/// `GET /servers/:id/commands/:commandId`: one receipt.
pub fn server_command_path(server_id: &ServerId, command_id: &FleetCommandId) -> String {
    format!(
        "{}/{}",
        server_commands_path(server_id),
        segment(command_id)
    )
}

/// `POST /servers/:id/commands/:commandId/cancel`.
pub fn command_cancellation_path(server_id: &ServerId, command_id: &FleetCommandId) -> String {
    format!("{}/cancel", server_command_path(server_id, command_id))
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;
use frontend_api_dtos::identifiers::{FleetCommandId, ServerId};

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::client::{api_get, api_post_keeping_refusal};
    use crate::endpoints::json_body;
    use crate::error::Result;
    use crate::token_provider::TokenProvider;
    use frontend_api_dtos::{FleetCommandList, FleetCommandReceipt, FleetCommandRequest};

    /// Request one command; the answer is its receipt, accepted and not yet performed.
    pub async fn request_fleet_command(
        store: impl TokenProvider,
        server_id: &ServerId,
        request: &FleetCommandRequest,
    ) -> Result<FleetCommandReceipt> {
        let path = server_commands_path(server_id);
        api_post_keeping_refusal(store, &path, json_body(request)?).await
    }

    /// The server's commands, newest first.
    pub async fn load_fleet_commands(
        store: impl TokenProvider,
        server_id: &ServerId,
    ) -> Result<FleetCommandList> {
        api_get(store, &server_commands_path(server_id)).await
    }

    /// One command's receipt as it stands now.
    pub async fn load_fleet_command(
        store: impl TokenProvider,
        server_id: &ServerId,
        command_id: &FleetCommandId,
    ) -> Result<FleetCommandReceipt> {
        api_get(store, &server_command_path(server_id, command_id)).await
    }

    /// Cancel a command no executor has claimed.
    pub async fn cancel_fleet_command(
        store: impl TokenProvider,
        server_id: &ServerId,
        command_id: &FleetCommandId,
    ) -> Result<FleetCommandReceipt> {
        let path = command_cancellation_path(server_id, command_id);
        api_post_keeping_refusal(store, &path, serde_json::json!({})).await
    }
}
