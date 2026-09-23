//! The HTTP client: one request contract, shared by every verb the app calls.
//!
//! **Role:** groups the request verbs with the failure types they return and the refresh policy
//! that keeps a session alive across them.
//! **Position:** the only route from the app to the backend. Pages and the editor import the verbs
//! from here and never build a request themselves.
//! **Signals & state:** the refresh module owns the per-tab single-flight cell and the peer-rotation
//! channel; nothing else here holds state.
//! **Invariants:** exactly one retry per request. A `401` is refreshed once through the shared
//! single flight and retried once with the rotated token; any other status, or a retry that is
//! still `401`, propagates to the caller — as a `(status, message)` pair from the plain verbs, and
//! as an [`ApiRefusal`] with its structured reason from the refusal-keeping ones. The request verbs
//! are browser-only, while the failure types and the refresh policy compile natively so the policy
//! can be tested without a browser.

pub mod errors;
pub mod fetched;
pub mod refresh;
pub mod refusals;
#[cfg(target_arch = "wasm32")]
pub mod requests;

#[allow(unused_imports)]
pub use errors::{
    api_error_message, error_body_message, split_error_lines, ApiErr, ApiFailure, MAX_ERROR_DETAILS,
};
#[allow(unused_imports)]
pub use fetched::Fetched;
#[allow(unused_imports)]
pub use refresh::{
    peer_rotation_supersedes, refresh_cross_tab, send_with_refresh, REFRESH_LOCK_NAME,
};
#[allow(unused_imports)]
pub use refusals::{decode_answer, ApiRefusal};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub use requests::{
    api_delete, api_delete_keeping_refusal, api_get, api_patch, api_patch_keeping_refusal,
    api_post, api_post_keeping_refusal, api_post_ok, api_post_raw, api_put,
    api_put_keeping_refusal, api_upload_file, bootstrap,
};

/// A pending request: resolves to the deserialised body, or to the failure pair.
pub type Req<T> = futures::future::LocalBoxFuture<'static, Result<T, ApiErr>>;

/// Path prefix every request is built on.
#[cfg(target_arch = "wasm32")]
pub(crate) const API_BASE: &str = "/api/v1";

#[cfg(test)]
#[path = "tests/client.rs"]
mod tests;
