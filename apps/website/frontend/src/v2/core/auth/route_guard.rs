//! Whether the current role may stay on the current route, and the effect that enforces it.
//!
//! **Role:** turns the route table's role requirements into a redirect decision, and drives that
//! decision from the live location.
//! **Position:** installed once when the session store is built, under the router, so the location
//! context is available.
//! **Signals & state:** reads the store's `user` and `bootstrapping` signals and the router's
//! current path; navigates and raises a notice when a route is refused.
//! **Invariants:** the decision is a pure function so it can be tested without a browser. It waits
//! out bootstrapping and refuses nothing while the profile is still being fetched, because a store
//! that has not loaded yet looks exactly like a signed-out one.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::store::AuthStore;
use crate::shell::nav_config::Role;

/// Where the viewer must be sent, if the current role may not stay on `path`.
///
/// Returns `None` while `bootstrapping`, so an authorised deep link is not bounced before the
/// profile has arrived.
pub fn route_auth_redirect(path: &str, role: Option<Role>, bootstrapping: bool) -> Option<String> {
    if bootstrapping {
        return None;
    }
    if crate::router::role_may_enter(path, role) {
        return None;
    }
    crate::router::auth_denial_redirect(path)
}

/// Watch the current route and redirect whenever [`route_auth_redirect`] says the viewer may not
/// stay.
///
/// Raises a notice through the toast context when one has been provided; a deep link that arrives
/// before the shell has mounted still gets the redirect, just without the message.
#[cfg(target_arch = "wasm32")]
pub(super) fn install_route_auth_guard(store: AuthStore) {
    // Called from `AuthStore::new()` inside `AppLayout` (under `<Router>`), so location context is live.
    let pathname = leptos_router::hooks::use_location().pathname;
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move |_| {
        let path = pathname.get();
        // Strip query/hash if a caller ever hands a full URL; location.pathname is path-only.
        let path = path.split('?').next().unwrap_or(path.as_str());
        let path = path.split('#').next().unwrap_or(path);
        let bootstrapping = store.bootstrapping.get();
        let role = store.user.get().map(|u| u.role);
        let Some(dest) = route_auth_redirect(path, role, bootstrapping) else {
            return;
        };
        // Role notice: toast when the shell has mounted Toasts; query param remains for deep links.
        if let Some(toasts) = use_context::<crate::v2::core::ui::toast::Toasts>() {
            toasts.message("Mission Maker role required to open the editor.");
        }
        navigate(&dest, Default::default());
    });
}
