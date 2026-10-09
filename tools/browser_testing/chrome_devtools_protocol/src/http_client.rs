//! The HTTP clients this crate builds, each built after the rustls crypto provider is installed.
//!
//! **Role:** installs the ring provider as the rustls process default, once, and hands out the
//! `reqwest` client every construction site uses.
//! **Position:** called by the browser launch, which polls the DevTools endpoint.
//! **Signals & state:** one process-wide `Once`; the rustls process default it sets.
//! **Invariants:** this crate's `reqwest` speaks plain HTTP on its own, but a build that unifies
//! features with a crate enabling `rustls-no-provider` (a workspace-wide `cargo test`, or a binary
//! linking an API crate) gives it a TLS backend with no provider, and a client built then panics;
//! every client built through here works in both builds. A provider another component installed
//! first stays installed (`install_default` refuses a second, and that refusal is not an error).

use std::sync::Once;

static TLS_INIT: Once = Once::new();

/// Install the ring provider as the rustls process default; every call after the first is a
/// no-op.
pub(crate) fn ensure_tls_provider() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// A default `reqwest` client, after [`ensure_tls_provider`].
pub(crate) fn new_http_client() -> reqwest::Client {
    ensure_tls_provider();
    reqwest::Client::new()
}
