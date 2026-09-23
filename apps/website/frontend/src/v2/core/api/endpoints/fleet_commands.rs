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
pub fn server_commands_path(server_id: &str) -> String {
    format!("/servers/{}/commands", segment(server_id))
}

/// `GET /servers/:id/commands/:commandId`: one receipt.
pub fn server_command_path(server_id: &str, command_id: &str) -> String {
    format!(
        "{}/{}",
        server_commands_path(server_id),
        segment(command_id)
    )
}

/// `POST /servers/:id/commands/:commandId/cancel`.
pub fn command_cancellation_path(server_id: &str, command_id: &str) -> String {
    format!("{}/cancel", server_command_path(server_id, command_id))
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{api_get, api_post_keeping_refusal, ApiErr, ApiRefusal};
    use crate::v2::core::api::dto::{FleetCommandList, FleetCommandReceipt, FleetCommandRequest};
    use crate::v2::core::api::endpoints::json_body;
    use crate::v2::core::auth::AuthStore;

    /// Request one command; the answer is its receipt, accepted and not yet performed.
    pub async fn request_fleet_command(
        store: AuthStore,
        server_id: &str,
        request: &FleetCommandRequest,
    ) -> Result<FleetCommandReceipt, ApiRefusal> {
        let path = server_commands_path(server_id);
        api_post_keeping_refusal(store, &path, json_body(request)?).await
    }

    /// The server's commands, newest first.
    pub async fn load_fleet_commands(
        store: AuthStore,
        server_id: &str,
    ) -> Result<FleetCommandList, ApiErr> {
        api_get(store, &server_commands_path(server_id)).await
    }

    /// One command's receipt as it stands now.
    pub async fn load_fleet_command(
        store: AuthStore,
        server_id: &str,
        command_id: &str,
    ) -> Result<FleetCommandReceipt, ApiErr> {
        api_get(store, &server_command_path(server_id, command_id)).await
    }

    /// Cancel a command no executor has claimed.
    pub async fn cancel_fleet_command(
        store: AuthStore,
        server_id: &str,
        command_id: &str,
    ) -> Result<FleetCommandReceipt, ApiRefusal> {
        let path = command_cancellation_path(server_id, command_id);
        api_post_keeping_refusal(store, &path, serde_json::json!({})).await
    }
}
