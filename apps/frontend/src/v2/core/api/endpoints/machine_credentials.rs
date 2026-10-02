//! The administrator routes of one server's machine credentials: list, issue and revoke.
//!
//! **Role:** the credential collection and revocation paths, and one call per route.
//! **Position:** called by the server control screen's credential panel.
//! **Signals & state:** none.
//! **Invariants:** the issue answer is the only place a secret ever appears, and the backend never
//! shows it again. A revocation names its reason in the query — it is required, and the backend
//! refuses a revocation without one — and answers the revoked credential; revoking one credential
//! leaves the server's others untouched.

use super::encode_path_segment as segment;

/// `GET` (list) and `POST` (issue) `/servers/:id/credentials`.
pub fn server_credentials_path(server_id: &str) -> String {
    format!("/servers/{}/credentials", segment(server_id))
}

/// `DELETE /servers/:id/credentials/:credentialId?reason=…`.
pub fn credential_revocation_path(server_id: &str, credential_id: &str, reason: &str) -> String {
    format!(
        "/servers/{}/credentials/{}?reason={}",
        segment(server_id),
        segment(credential_id),
        segment(reason)
    )
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{
        api_delete_keeping_refusal, api_get, api_post_keeping_refusal, ApiErr, ApiRefusal,
    };
    use crate::v2::core::api::dto::{
        IssuedMachineCredential, MachineCredential, MachineCredentialIssue, MachineCredentialList,
    };
    use crate::v2::core::api::endpoints::json_body;
    use crate::v2::core::auth::AuthStore;

    /// Every credential the server has had, live and revoked.
    pub async fn load_machine_credentials(
        store: AuthStore,
        server_id: &str,
    ) -> Result<MachineCredentialList, ApiErr> {
        api_get(store, &server_credentials_path(server_id)).await
    }

    /// Issue a credential; the answer carries the secret, once.
    pub async fn issue_machine_credential(
        store: AuthStore,
        server_id: &str,
        issue: &MachineCredentialIssue,
    ) -> Result<IssuedMachineCredential, ApiRefusal> {
        let path = server_credentials_path(server_id);
        api_post_keeping_refusal(store, &path, json_body(issue)?).await
    }

    /// Revoke one credential with the reason the administrator gave.
    pub async fn revoke_machine_credential(
        store: AuthStore,
        server_id: &str,
        credential_id: &str,
        reason: &str,
    ) -> Result<MachineCredential, ApiRefusal> {
        let path = credential_revocation_path(server_id, credential_id, reason);
        api_delete_keeping_refusal(store, &path).await
    }
}
