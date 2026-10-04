//! The session itself: the minted session, and the persisted slice of it.
//!
//! **Role:** the plain data of a session, plus the serialisation of the part that survives a page
//! reload.
//! **Position:** read by the store above it and by the session refresh; the profile and the
//! rotated token pair it is built from are the transport's wire types
//! ([`frontend_api_dtos::User`],
//! [`frontend_api_dtos::RefreshResponse`]).
//! **Signals & state:** none of its own. The persist functions read and write one browser-storage
//! key.
//! **Invariants:** the access token is never persisted — only the refresh token, the profile and
//! the expiry. The persisted blob's key names are fixed; renaming one orphans every signed-in
//! session on the next release.

use serde::{Deserialize, Serialize};

use frontend_api_dtos::User;
use frontend_api_dtos::identifiers::AuthenticationSessionId;

/// A minted session: the access token, the rotating refresh token, when the access token expires,
/// the profile, and whether a game account is linked. The store's native tests install one
/// directly; no shipped path builds one.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// The short-lived bearer token.
    pub access_token: String,
    /// The single-use refresh token that rotates on every refresh.
    pub refresh_token: String,
    /// When the access token expires, as an RFC 3339 instant.
    pub expires_at: String,
    /// The signed-in member's profile.
    pub user: User,
    /// Whether a game account is linked to the member.
    pub arma_linked: bool,
}

/// Browser-storage key the session slice is persisted under.
pub const AUTH_PERSIST_KEY: &str = "tbd-auth";

/// The part of a session that survives a reload: the refresh token, the profile, and the expiry.
///
/// Deliberately not the access token. Its absence is what keeps a short-lived credential out of
/// durable storage, and a test pins that absence.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistState {
    /// Untrusted session correlation hint; only the API authorizes credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<AuthenticationSessionId>,
    /// The single-use refresh token a cold start spends to mint a new access token.
    pub refresh_token: Option<String>,
    /// The signed-in member's profile, shown before the first refresh lands.
    pub user: Option<User>,
    /// When the access token that came with this refresh token expires, as an RFC 3339 instant.
    pub expires_at: Option<String>,
}

/// The full stored blob: the persisted slice plus the schema version it was written at.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedAuth {
    /// The persisted slice.
    pub state: PersistState,
    /// The schema version the blob was written at.
    pub version: u32,
}

/// Serialise the persisted slice to the exact blob string. Pure — the browser half simply writes
/// the result.
pub fn to_persist_json(state: &PersistState) -> String {
    serde_json::to_string(&PersistedAuth {
        state: state.clone(),
        version: 0,
    })
    .unwrap_or_default()
}

/// Parse a stored blob back into the persisted slice, or `None` when it cannot be read.
pub fn from_persist_json(json: &str) -> Option<PersistState> {
    serde_json::from_str::<PersistedAuth>(json)
        .ok()
        .map(|p| p.state)
}

/// Adopt a profile for the same session and account, retaining any newer rotated credential.
/// The current refresh credential and expiry remain authoritative; profile reads cannot rotate them.
pub fn profile_persistence_update(
    current: &PersistState,
    proposed: &PersistState,
) -> Option<PersistState> {
    let token = current
        .refresh_token
        .as_deref()
        .filter(|token| !token.is_empty())?;
    if !super::session_identity::same_session(
        current.session_id.as_ref().map(|id| id.as_str()),
        proposed.session_id.as_ref().map(|id| id.as_str()),
    ) && (proposed.refresh_token.as_deref() != Some(token)
        || current.session_id != proposed.session_id)
    {
        return None;
    }
    proposed
        .refresh_token
        .as_deref()
        .filter(|value| !value.is_empty())?;
    let profile = proposed.user.as_ref()?;
    if current
        .user
        .as_ref()
        .is_some_and(|user| user.discord_id != profile.discord_id)
    {
        return None;
    }
    let mut updated = current.clone();
    updated.user = Some(profile.clone());
    Some(updated)
}

/// Check and persist a profile while holding the shared authentication Web Lock.
/// Returns false when locking/storage is unavailable or the current session differs from the proposal.
#[cfg(target_arch = "wasm32")]
pub async fn persist_profile_if_current(state: &PersistState) -> bool {
    use futures::future::FutureExt;
    let proposed = state.clone();
    crate::session_refresh::with_refresh_lock(
        async move {
            let Some(storage) =
                web_sys::window().and_then(|window| window.local_storage().ok().flatten())
            else {
                return false;
            };
            let Ok(Some(json)) = storage.get_item(AUTH_PERSIST_KEY) else {
                return false;
            };
            let Some(current) = from_persist_json(&json) else {
                return false;
            };
            let Some(updated) = profile_persistence_update(&current, &proposed) else {
                return false;
            };
            storage
                .set_item(AUTH_PERSIST_KEY, &to_persist_json(&updated))
                .is_ok()
        }
        .boxed_local(),
    )
    .await
}

/// Write the persisted slice and report storage failure to the credential owner.
#[cfg(target_arch = "wasm32")]
pub fn persist(state: &PersistState) -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .is_some_and(|storage| {
            storage
                .set_item(AUTH_PERSIST_KEY, &to_persist_json(state))
                .is_ok()
        })
}

/// Remove the current credential under the shared Web Lock before spending it or logging out.
#[cfg(target_arch = "wasm32")]
pub fn clear_persisted() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .is_some_and(|storage| storage.remove_item(AUTH_PERSIST_KEY).is_ok())
}

/// Logout clears rotated successors of the departing session, but preserves a new login.
pub fn persisted_belongs_to_session(current: &PersistState, departing: &PersistState) -> bool {
    super::session_identity::same_session(
        current.session_id.as_ref().map(|id| id.as_str()),
        departing.session_id.as_ref().map(|id| id.as_str()),
    ) || current
        .refresh_token
        .as_deref()
        .filter(|value| !value.is_empty())
        .is_some_and(|token| departing.refresh_token.as_deref() == Some(token))
}

/// Read the persisted slice back out of browser storage on a cold start.
#[cfg(target_arch = "wasm32")]
pub fn load_persisted() -> Option<PersistState> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(AUTH_PERSIST_KEY).ok()??;
    from_persist_json(&json)
}

#[cfg(test)]
#[path = "tests/session_persistence.rs"]
mod tests;
