//! The HTTP client: one request contract, shared by every verb the app calls.
//!
//! **Role:** groups the request verbs with the error body reader and the settled-fetch wrapper,
//! the refresh policy that keeps a session alive across them, and the single-flight cell type that
//! policy shares a refresh through.
//! **Position:** the only route from the app to the backend. Pages and the editor import the verbs
//! from here and never build a request themselves; every verb takes the session as a
//! `TokenProvider`, whose per-tab cell and locked refresh live in the session layer above.
//! **Signals & state:** none; the per-tab single-flight cell and the peer-rotation channel are the
//! token provider's.
//! **Invariants:** exactly one retry per request. A `401` is refreshed once through the shared
//! single flight and retried once with the rotated token; any other status, or a retry that is
//! still `401`, propagates to the caller as the crate's [`crate::Error`] — with the structured
//! reason its body names from the refusal-keeping verbs. The request verbs are browser-only, while
//! the failure readers and the refresh policy also compile into the native test build, so the
//! policy can be tested without a browser.

pub mod errors;
pub mod fetched;
pub mod rate_limit_retry;
pub mod refresh;
pub mod refusals;
#[cfg(target_arch = "wasm32")]
pub mod requests;
pub mod single_flight;

pub use errors::MAX_ERROR_DETAILS;
pub use errors::{error_body_message, split_error_lines};
pub use fetched::Fetched;
pub use refresh::peer_rotation_supersedes;
#[cfg(test)]
pub use refresh::refresh_cross_tab;
#[cfg(test)]
pub use refresh::send_with_refresh;
#[cfg(target_arch = "wasm32")]
pub use requests::{
    api_delete, api_delete_keeping_refusal, api_get, api_patch, api_patch_keeping_refusal,
    api_post, api_post_form_keeping_refusal, api_post_keeping_refusal, api_post_ok, api_post_raw,
    api_put, api_put_keeping_refusal, api_upload_file,
};
pub use single_flight::SingleFlight;

/// A pending request: resolves to the deserialised body, or to the crate's [`crate::Error`].
pub type Req<T> = futures::future::LocalBoxFuture<'static, crate::Result<T>>;

/// The path prefix every request is built on, for the callers that build a request of their own
/// (the session's refresh request).
pub const API_BASE: &str = "/api/v1";

#[cfg(test)]
#[path = "tests/client.rs"]
mod tests;

#[cfg(target_arch = "wasm32")]
pub mod public_reads;
