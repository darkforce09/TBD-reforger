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
pub fn server_credentials_path(server_id: &ServerId) -> String {
    format!("/servers/{}/credentials", segment(server_id))
}

/// `DELETE /servers/:id/credentials/:credentialId?reason=…`.
pub fn credential_revocation_path(
    server_id: &ServerId,
    credential_id: &MachineCredentialId,
    reason: &str,
) -> String {
    format!(
        "/servers/{}/credentials/{}?reason={}",
        segment(server_id),
        segment(credential_id),
        segment(reason)
    )
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;
use frontend_api_dtos::identifiers::{MachineCredentialId, ServerId};

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::client::{api_delete_keeping_refusal, api_get, api_post_keeping_refusal};
    use crate::endpoints::json_body;
    use crate::error::Result;
    use crate::token_provider::TokenProvider;
    use frontend_api_dtos::{
        IssuedMachineCredential, MachineCredential, MachineCredentialIssue, MachineCredentialList,
    };

    /// Every credential the server has had, live and revoked.
    pub async fn load_machine_credentials(
        store: impl TokenProvider,
        server_id: &ServerId,
    ) -> Result<MachineCredentialList> {
        api_get(store, &server_credentials_path(server_id)).await
    }

    /// Issue a credential; the answer carries the secret, once.
    pub async fn issue_machine_credential(
        store: impl TokenProvider,
        server_id: &ServerId,
        issue: &MachineCredentialIssue,
    ) -> Result<IssuedMachineCredential> {
        let path = server_credentials_path(server_id);
        api_post_keeping_refusal(store, &path, json_body(issue)?).await
    }

    /// Revoke one credential with the reason the administrator gave.
    pub async fn revoke_machine_credential(
        store: impl TokenProvider,
        server_id: &ServerId,
        credential_id: &MachineCredentialId,
        reason: &str,
    ) -> Result<MachineCredential> {
        let path = credential_revocation_path(server_id, credential_id, reason);
        api_delete_keeping_refusal(store, &path).await
    }
}
