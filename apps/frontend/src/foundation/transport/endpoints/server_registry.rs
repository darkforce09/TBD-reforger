//! The administrator routes of the game-server registry: list, register, change and deactivate a
//! server, and the modpacks a registration may require.
//!
//! **Role:** the server collection and single-server paths, the modpack list the registration
//! form offers, and one call per route.
//! **Position:** called by the server control screen's server registry, and by the event
//! manager's forms, which offer the servers an operation can be scheduled on.
//! **Signals & state:** none.
//! **Invariants:** the server id is path data and is percent-encoded like any other. A
//! registration and a change answer the server's row in the shape `GET /servers` lists, so the
//! caller puts it straight into the list it renders. A deactivation answers 204 with no body, so
//! its call reads no answer: it sets `is_active` to false and changes nothing else, and a change
//! with `is_active: true` reverses it.

#[cfg(any(target_arch = "wasm32", test))]
use super::encode_path_segment as segment;

/// `GET` (list) and `POST` (register) `/servers`.
#[cfg(any(target_arch = "wasm32", test))]
pub fn servers_path() -> String {
    "/servers".to_string()
}

/// `PATCH` (change) and `DELETE` (deactivate) `/servers/:id`.
#[cfg(any(target_arch = "wasm32", test))]
pub fn server_path(server_id: &str) -> String {
    format!("/servers/{}", segment(server_id))
}

/// `GET /modpacks`: every modpack, the choices of a server's required modpack.
#[cfg(any(target_arch = "wasm32", test))]
pub fn required_modpack_choices_path() -> String {
    "/modpacks".to_string()
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::foundation::transport::client::{
        api_delete, api_get, api_patch_keeping_refusal, api_post_keeping_refusal, ApiErr,
        ApiRefusal,
    };
    use crate::foundation::transport::dto::{
        DataEnvelope, ModpackDto, ServerChange, ServerRegistration, ServerRowDto,
    };
    use crate::foundation::transport::endpoints::json_body;
    use crate::foundation::transport::token_provider::TokenProvider;

    /// Every server the viewer may see; an administrator also sees the deactivated ones.
    pub async fn load_servers(
        store: impl TokenProvider,
    ) -> Result<DataEnvelope<ServerRowDto>, ApiErr> {
        api_get(store, &servers_path()).await
    }

    /// Register a game server; the answer is its row, active unless the body said otherwise.
    pub async fn register_server(
        store: impl TokenProvider,
        registration: &ServerRegistration,
    ) -> Result<ServerRowDto, ApiRefusal> {
        api_post_keeping_refusal(store, &servers_path(), json_body(registration)?).await
    }

    /// Change the fields of a server's registration the body names; the answer is its row.
    pub async fn change_server(
        store: impl TokenProvider,
        server_id: &str,
        change: &ServerChange,
    ) -> Result<ServerRowDto, ApiRefusal> {
        let path = server_path(server_id);
        api_patch_keeping_refusal(store, &path, json_body(change)?).await
    }

    /// Deactivate a server: it stays listed, and a change with `is_active: true` reactivates it.
    pub async fn deactivate_server(
        store: impl TokenProvider,
        server_id: &str,
    ) -> Result<(), ApiRefusal> {
        api_delete(store, &server_path(server_id))
            .await
            .map_err(ApiRefusal::from)
    }

    /// Every modpack, for the registration form's required-modpack choice.
    pub async fn load_required_modpack_choices(
        store: impl TokenProvider,
    ) -> Result<DataEnvelope<ModpackDto>, ApiErr> {
        api_get(store, &required_modpack_choices_path()).await
    }
}
