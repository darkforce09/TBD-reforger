//! Work that runs when a session ends: the hooks a higher layer registers, each run with the
//! departing account's id.
//!
//! **Role:** holds the registry of sign-out hooks and runs every registered hook, in registration
//! order, with the Discord id of the account that is leaving.
//! **Position:** compiled for `wasm32` and for the native tests. The application root registers its
//! hooks at start, before the app mounts (the Mission Creator's purge of the departing account's
//! local drafts), and [`AuthStore::clear_session`](super::store::AuthStore::clear_session) runs
//! them, so the session ends that work without importing any layer above it.
//! **Signals & state:** one thread-local list of plain function pointers, so one registry per
//! browsing context (per test thread natively). Not a Leptos signal.
//! **Invariants:** a hook receives the id of the account that is leaving, read before the session
//! signals are cleared; the store never runs the hooks for an absent or blank id. A hook is a plain
//! `fn(&str)` and captures nothing, so it cannot keep a departed page's component or signal alive.
//! A hook is synchronous; work that must await spawns its own task.

use std::cell::RefCell;

/// A sign-out hook: called with the departing account's Discord id.
pub type LogoutHook = fn(&str);

thread_local! {
    /// The registered hooks, in registration order.
    static LOGOUT_HOOKS: RefCell<Vec<LogoutHook>> = const { RefCell::new(Vec::new()) };
}

/// Register `hook` to run whenever a session of a known account ends.
pub fn register_logout_hook(hook: LogoutHook) {
    LOGOUT_HOOKS.with(|hooks| hooks.borrow_mut().push(hook));
}

/// Run every registered hook with `departing`, in registration order.
///
/// The list is copied out before the first hook runs, so a hook that registers another hook does
/// not re-enter the registry's borrow.
pub(super) fn run_logout_hooks(departing: &str) {
    let hooks = LOGOUT_HOOKS.with(|hooks| hooks.borrow().clone());
    for hook in hooks {
        hook(departing);
    }
}

#[cfg(test)]
#[path = "tests/logout_hooks.rs"]
mod tests;
