//! Account-scoped routes: sign-in, the OAuth callback, and user settings.
//!
//! **Role:** groups the pages that act on the viewer's own session rather than on mission or
//! operations data.
//! **Position:** sign-in and the redirect that returns from it are rendered bare, outside the
//! navigation frame; the settings page renders inside it.
//! **Signals & state:** the pages here read and write the shared authentication store.
//! **Invariants:** these routes must stay reachable while signed out — the callback page in
//! particular runs before a session exists.

pub mod auth_callback;
pub mod login;
