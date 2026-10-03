//! The failures of the Discord OAuth2, guild-member and webhook clients.
//!
//! **Role:** one typed error for every call of the Discord clients that can fail with something
//! other than a membership lookup outcome.
//! **Position:** `api_discord`; returned by the OAuth2 and guild-member client and by the
//! announcement webhook; the OAuth handlers of `api_identity_and_access` classify it by variant and
//! log it with its causes, the community content push records any failure as a CRIT audit row.
//! **Signals & state:** none; plain values.
//! **Invariants:** each message is the text the clients have always reported; a transport or
//! decode failure is the transparent `reqwest` or `serde_json` error, so its message and its
//! causes read exactly as that library renders them; no variant carries a token, a secret or an
//! authorization code.

/// A failed Discord OAuth2, guild-member or webhook call.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The OAuth2 application has no client id, so no consent URL can be built.
    #[error("discord: client_id not configured")]
    ClientIdNotConfigured,
    /// The token exchange answered 2xx without an access token.
    #[error("discord: empty access token")]
    EmptyAccessToken,
    /// The guild-member read answered 404 with a code other than Discord's unknown-member code.
    #[error("discord: membership verification unavailable")]
    MembershipVerificationUnavailable,
    /// Discord answered a non-2xx status; `body` holds at most 4096 characters of its body.
    #[error("discord: status {status}: {body}")]
    DiscordStatus {
        /// The HTTP status code.
        status: u16,
        /// The start of the response body.
        body: String,
    },
    /// The webhook URL is empty, so pushing is disabled.
    #[error("webhook not configured")]
    WebhookNotConfigured,
    /// The webhook answered a non-2xx status; `body` holds at most 4096 characters of its body.
    #[error("webhook push: status {status}: {body}")]
    WebhookStatus {
        /// The HTTP status code.
        status: u16,
        /// The start of the response body.
        body: String,
    },
    /// The request could not be sent, or its answer could not be read or decoded.
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    /// A JSON payload could not be encoded, or a JSON answer could not be decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// The result of a Discord client call.
pub type Result<T> = std::result::Result<T, Error>;
