//! The page the sign-in redirect lands on.
//!
//! **Role:** groups the callback page with the fragment parsing it needs.
//! **Position:** the redirect target of the sign-in flow, reached only by the identity provider.
//! **Signals & state:** the page writes the session store; nothing is held at this level.
//! **Invariants:** the route must stay reachable while signed out — it runs before a session
//! exists.

mod page;

pub use page::AuthCallbackPage;
