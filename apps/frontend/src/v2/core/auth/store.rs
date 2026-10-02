//! The reactive session store: the signals every component reads the current session from.
//!
//! **Role:** holds the session as Leptos signals, provides the mutations that start, rotate and end
//! it, and answers the role questions the interface gates on.
//! **Position:** provided once at the application root and read through context everywhere else.
//! **Signals & state:** owns `access_token`, `refresh_token`, `expires_at`, `user` and
//! `bootstrapping`, membership status, session generation and profile request order. Ending a
//! session also purges the departing account's locally stored documents.
//! **Invariants:** every field is a signal, so the store is `Copy` and threads through views
//! without cloning. `bootstrapping` is true from construction until the profile fetch settles,
//! and the route guard must wait it out or an authorised deep link is bounced on its first frame.
//! A profile response belongs to one session generation and the latest started profile request,
//! and adopting it writes only the signals it changes, so a poll that returns the same profile
//! notifies no subscriber. Requests wait for the cold-start session restore ([`SessionRestore`]),
//! which settles once the store knows its session: restored from storage, found absent, signed in
//! or ended.

use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::route_guard::install_route_auth_guard;
use super::session::{PersistState, RefreshResponse, Session, User};
use super::session_restore::SessionRestore;
use crate::v2::core::api::dto::MeResponse;
use crate::v2::core::auth::{has_min_role, has_min_role_authed, Role};

/// The session and request ordering captured before an asynchronous profile fetch begins.
///
/// Only the store constructs this token, so consumers cannot relabel an obsolete response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileRequest {
    generation: u64,
    order: u64,
}

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
    pub membership_stale: RwSignal<bool>,
    pub membership_override_active: RwSignal<bool>,
    pub can_manage_membership_override: RwSignal<bool>,
    session_generation: RwSignal<u64>,
    session_id: RwSignal<Option<String>>,
    profile_request_order: RwSignal<u64>,
    session_restore: SessionRestore,
}

#[allow(dead_code)]
impl AuthStore {
    /// Build the store and, in the browser, install the route guard.
    ///
    /// Starts unresolved in the browser until the shared, locked credential read settles, so
    /// a peer's in-flight rotation cannot bounce an authorized deep link.
    pub fn new() -> Self {
        // Shared storage is temporarily empty during another tab's rotation. Only the locked
        // bootstrap read can settle whether this browser is authenticated.
        #[cfg(target_arch = "wasm32")]
        let bootstrapping = true;
        #[cfg(not(target_arch = "wasm32"))]
        let bootstrapping = false;
        let store = Self {
            access_token: RwSignal::new(None),
            refresh_token: RwSignal::new(None),
            expires_at: RwSignal::new(None),
            user: RwSignal::new(None),
            bootstrapping: RwSignal::new(bootstrapping),
            membership_stale: RwSignal::new(false),
            membership_override_active: RwSignal::new(false),
            can_manage_membership_override: RwSignal::new(false),
            session_generation: RwSignal::new(0),
            session_id: RwSignal::new(None),
            profile_request_order: RwSignal::new(0),
            session_restore: SessionRestore::new(!bootstrapping),
        };
        #[cfg(target_arch = "wasm32")]
        install_route_auth_guard(store);
        store
    }

    /// Adopt a freshly minted session and leave the bootstrapping state.
    pub fn set_session(&self, s: Session) {
        self.advance_session_generation();
        self.reset_membership_status();
        self.session_id
            .set(super::session_identity::access_token_session_id(
                &s.access_token,
            ));
        self.access_token.set(Some(s.access_token));
        self.refresh_token.set(Some(s.refresh_token));
        self.expires_at.set(Some(s.expires_at));
        self.user.set(Some(s.user));
        self.bootstrapping.set(false);
        self.session_restore.settle();
    }

    /// The current session generation, captured before starting asynchronous session work.
    pub fn current_generation(&self) -> u64 {
        self.session_generation.get_untracked()
    }

    /// Whether asynchronous work still belongs to the session that initiated it.
    pub fn is_current_generation(&self, generation: u64) -> bool {
        self.current_generation() == generation
    }

    /// Wait until the store knows which session requests are sent under.
    pub async fn session_restored(&self) {
        self.session_restore.wait().await;
    }

    /// Settle the cold-start restore when there is no stored session to restore.
    pub fn settle_session_restore(&self) {
        self.session_restore.settle();
    }

    /// Start a profile request, superseding earlier requests even before this one finishes.
    pub fn begin_profile_request(&self) -> ProfileRequest {
        self.profile_request_order.update(|order| {
            *order = order
                .checked_add(1)
                .expect("profile request order exhausted");
        });
        ProfileRequest {
            generation: self.current_generation(),
            order: self.profile_request_order.get_untracked(),
        }
    }

    /// Adopt the latest requested profile only while its session and account remain current.
    ///
    /// Token rotation preserves the generation, so an in-flight profile may finish after a
    /// refresh. Logout and a freshly minted session invalidate all earlier profile responses.
    pub fn adopt_profile(&self, request: ProfileRequest, profile: &MeResponse) -> bool {
        if !self.is_current_generation(request.generation)
            || request.order != self.profile_request_order.get_untracked()
            || self.access_token.get_untracked().is_none()
            || self.user.with_untracked(|current| {
                current
                    .as_ref()
                    .is_some_and(|user| user.discord_id != profile.user.discord_id)
            })
        {
            return false;
        }
        self.set_profile(profile);
        true
    }

    fn advance_session_generation(&self) {
        self.session_generation.update(|generation| {
            *generation = generation
                .checked_add(1)
                .expect("session generation exhausted");
        });
    }

    fn reset_membership_status(&self) {
        self.membership_stale.set(false);
        self.membership_override_active.set(false);
        self.can_manage_membership_override.set(false);
    }

    /// Apply an authoritative profile after request ownership has been checked.
    ///
    /// Writes only the signals whose value the profile changes. The profile is re-polled while a
    /// page is open, and a signal write notifies its subscribers even when the value is equal, so
    /// an unconditional write would re-run every view that reads the session on every poll.
    fn set_profile(&self, profile: &MeResponse) {
        set_if_changed(self.user, Some(profile.user.clone()));
        set_if_changed(
            self.membership_stale,
            profile.membership_stale.unwrap_or(false),
        );
        set_if_changed(
            self.membership_override_active,
            profile.membership_override_active.unwrap_or(false),
        );
        set_if_changed(
            self.can_manage_membership_override,
            profile.can_manage_membership_override.unwrap_or(false),
        );
    }

    /// Restore a persisted correlation hint before the first authenticated request.
    pub fn restore_persisted(&self, persisted: PersistState) {
        self.advance_session_generation();
        self.reset_membership_status();
        self.access_token.set(None);
        self.session_id.set(persisted.session_id);
        self.refresh_token.set(persisted.refresh_token);
        self.expires_at.set(persisted.expires_at);
        self.user.set(persisted.user);
        self.session_restore.settle();
    }

    /// The untrusted session correlation hint; authorization is always performed by the API.
    pub fn current_session_id(&self) -> Option<String> {
        self.session_id.get_untracked()
    }

    /// Adopt a rotated pair. A different session invalidates all earlier profile work.
    ///
    /// Refresh tokens are single-use, so after any successful rotation the new one must be stored
    /// even when no profile is loaded yet, or the session dies at the next refresh.
    pub fn set_tokens(&self, t: RefreshResponse) {
        let next_id = super::session_identity::access_token_session_id(&t.access_token);
        let previous_id = self.session_id.get_untracked();
        if (previous_id.is_some() && previous_id != next_id)
            || (next_id.is_none() && self.access_token.get_untracked().is_some())
        {
            self.advance_session_generation();
            self.reset_membership_status();
            self.user.set(None);
        }
        self.session_id.set(next_id);
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
        self.advance_session_generation();
        #[cfg(target_arch = "wasm32")]
        let departing = self
            .user
            .get_untracked()
            .map(|u| u.discord_id)
            .filter(|id| !id.trim().is_empty());

        self.access_token.set(None);
        self.session_id.set(None);
        self.refresh_token.set(None);
        self.expires_at.set(None);
        self.user.set(None);
        self.reset_membership_status();
        self.bootstrapping.set(false);
        self.session_restore.settle();

        #[cfg(target_arch = "wasm32")]
        if let Some(owner) = departing {
            crate::v2::apps::editor::shell::hydrate::purge_local_documents(&owner);
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
            session_id: self.current_session_id(),
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

/// Store `next` in `signal` only when it differs from the current value.
///
/// `RwSignal::set` notifies every subscriber unconditionally; this notifies them only when the
/// stored value actually changes.
fn set_if_changed<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, next: T) {
    signal.maybe_update(|current| {
        let changed = *current != next;
        if changed {
            *current = next;
        }
        changed
    });
}

#[cfg(test)]
#[path = "tests/store.rs"]
mod tests;
