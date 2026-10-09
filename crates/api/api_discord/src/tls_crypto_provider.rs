//! The process-wide rustls crypto provider both Discord clients need before they build a client.
//!
//! **Role:** installs the ring provider as the rustls process default, once.
//! **Position:** `api_discord`; [`crate::discord_client::DiscordService`] and
//! [`crate::discord_webhook::WebhookService`] call [`ensure_tls_provider`] in their constructors.
//! **Signals & state:** one process-wide `Once`; the rustls process default it sets.
//! **Invariants:** the workspace builds rustls without a default provider (no aws-lc-rs C build),
//! so a `reqwest` client built before this call cannot speak HTTPS; a provider another component
//! installed first stays installed (`install_default` refuses a second, and that refusal is not
//! an error here).

use std::sync::Once;

static TLS_INIT: Once = Once::new();

/// Install the ring provider as the rustls process default; every call after the first is a
/// no-op.
pub(crate) fn ensure_tls_provider() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
