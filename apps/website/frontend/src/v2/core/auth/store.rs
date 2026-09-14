//! The reactive session store: the signals every component reads the current session from.
//!
//! **Role:** holds the session as Leptos signals, provides the mutations that start, rotate and end
//! it, and answers the role questions the interface gates on.
//! **Position:** provided once at the application root and read through context everywhere else.
//! **Signals & state:** owns `access_token`, `refresh_token`, `expires_at`, `user` and
//! `bootstrapping`. Ending a session also purges the departing account's locally stored documents.
//! **Invariants:** every field is a signal, so the store is `Copy` and threads through views
//! without cloning. `bootstrapping` is true from construction until the profile fetch settles,
//! and the route guard must wait it out or an authorised deep link is bounced on its first frame.

use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::route_guard::install_route_auth_guard;
#[cfg(target_arch = "wasm32")]
use super::session::load_persisted;
use super::session::{PersistState, RefreshResponse, Session, User};
use crate::shell::nav_config::{has_min_role, has_min_role_authed, Role};

/// The session as signals, provided at the application root and read through context.
///
/// Every field is an `RwSignal`, which makes the whole store `Copy`.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct AuthStore {
    pub access_token: RwSignal<Option<String>>,
    pub refresh_token: RwSignal<Option<String>>,
    pub expires_at: RwSignal<Option<String>>,
    pub user: RwSignal<Option<User>>,
    pub bootstrapping: RwSignal<bool>,
}

#[allow(dead_code)]
impl AuthStore {
    /// Build the store and, in the browser, install the route guard.
    ///
    /// Starts in the bootstrapping state when a refresh token is already on disk, so the guard
    /// cannot see a signed-out-looking store for one frame and bounce an authorised deep link
    /// before the profile has been fetched.
    pub fn new() -> Self {
        // A refresh token already on disk means a session is about to be restored, so start in
        // the bootstrapping state until the profile fetch finishes.
        #[cfg(target_arch = "wasm32")]
        let bootstrapping = load_persisted().and_then(|p| p.refresh_token).is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let bootstrapping = false;
        let store = Self {
            access_token: RwSignal::new(None),
            refresh_token: RwSignal::new(None),
            expires_at: RwSignal::new(None),
            user: RwSignal::new(None),
            bootstrapping: RwSignal::new(bootstrapping),
        };
        #[cfg(target_arch = "wasm32")]
        install_route_auth_guard(store);
        store
    }

    /// Adopt a freshly minted session and leave the bootstrapping state.
    pub fn set_session(&self, s: Session) {
        self.access_token.set(Some(s.access_token));
        self.refresh_token.set(Some(s.refresh_token));
        self.expires_at.set(Some(s.expires_at));
        self.user.set(Some(s.user));
        self.bootstrapping.set(false);
    }

    /// Adopt a rotated pair without touching the profile.
    ///
    /// Refresh tokens are single-use, so after any successful rotation the new one must be stored
    /// even when no profile is loaded yet, or the session dies at the next refresh.
    pub fn set_tokens(&self, t: RefreshResponse) {
        self.access_token.set(Some(t.access_token));
        self.refresh_token.set(Some(t.refresh_token));
        self.expires_at.set(Some(t.expires_at));
    }

    /// End the session, and delete the local documents filed under the departing account.
    ///
    /// The purge lives here rather than on a sign-out button because this is the function that
    /// destroys the identity those records are filed under, and several paths can end a session.
    ///
    /// The account id is read off the signal *before* the clear, deliberately. The storage layer
    /// resolves its owner from the persisted blob, and a sign-out clears these signals and that
    /// blob moments apart; asking afterwards would name the anonymous owner instead, deleting a
    /// signed-out visitor's own drafts while leaving the departing account's in place. An absent
    /// or blank id therefore purges nothing at all.
    pub fn clear_session(&self) {
        #[cfg(target_arch = "wasm32")]
        let departing = self
            .user
            .get_untracked()
            .map(|u| u.discord_id)
            .filter(|id| !id.trim().is_empty());

        self.access_token.set(None);
        self.refresh_token.set(None);
        self.expires_at.set(None);
        self.user.set(None);
        self.bootstrapping.set(false);

        #[cfg(target_arch = "wasm32")]
        if let Some(owner) = departing {
            crate::editor::state::hydrate::purge_local_documents(&owner);
        }
    }

    /// True when there is both an access token and a loaded profile.
    pub fn is_authenticated(&self) -> bool {
        self.access_token.get().is_some() && self.user.get().is_some()
    }

    /// Whether the current role meets `min`, treating a signed-out viewer as the guest role.
    pub fn has_min_role(&self, min: Role) -> bool {
        has_min_role(self.user.get().map(|u| u.role), min)
    }

    /// Whether the current role meets `min`, with a signed-out viewer never passing.
    ///
    /// This is the gate for actions and protected routes, where "not signed in" must fail rather
    /// than fall back to the guest tier.
    pub fn has_min_role_authed(&self, min: Role) -> bool {
        has_min_role_authed(self.user.get().map(|u| u.role), min)
    }

    /// The slice of this store that is written to browser storage.
    pub fn persist_state(&self) -> PersistState {
        PersistState {
            refresh_token: self.refresh_token.get_untracked(),
            user: self.user.get_untracked(),
            expires_at: self.expires_at.get_untracked(),
        }
    }
}

/// Construct the store the same way [`AuthStore::new()`] does.
#[allow(dead_code)]
impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}
