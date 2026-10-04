//! Anonymous GET requests, which never read or mutate authentication state.
//!
//! **Role:** the one verb for the public reads (the debug benches' catalog and agreement reads):
//! a credential-free GET that honours an abort signal and the API's rate-limit answers.
//! **Position:** beside the authenticated verbs of [`super::requests`]; it takes no token provider.
//! **Signals & state:** a per-thread count of the public reads in flight.
//! **Invariants:** at most four public reads run at once; a path must be a site path (`/…`, never
//! `//host`); the request carries no credentials.
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

thread_local! { static ACTIVE:std::cell::Cell<usize>=const{std::cell::Cell::new(0)}; }
struct RequestSlot;
impl Drop for RequestSlot {
    fn drop(&mut self) {
        ACTIVE.with(|n| n.set(n.get().saturating_sub(1)));
    }
}
async fn acquire(signal: &web_sys::AbortSignal) -> Result<RequestSlot> {
    loop {
        if signal.aborted() {
            return Err(Error::Cancelled);
        }
        if ACTIVE.with(|n| {
            if n.get() < 4 {
                n.set(n.get() + 1);
                true
            } else {
                false
            }
        }) {
            return Ok(RequestSlot);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
}

/// `GET` the API path `path` without credentials and decode its JSON body as `T`.
///
/// Waits for one of four slots and retries the API's rate-limit answers.
///
/// # Errors
///
/// [`Error::PublicRefusal`] with the API's error sentence on a non-2xx answer,
/// [`Error::Cancelled`] once `signal` aborts, [`Error::NotASitePath`] for a path that is not a site
/// path, and [`Error::Failed`] with the browser's reason when the send or the decode fails.
pub async fn public_get<T: DeserializeOwned>(
    path: &str,
    signal: &web_sys::AbortSignal,
) -> Result<T> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err(Error::NotASitePath);
    }
    let _slot = acquire(signal).await?;
    let url = format!("{}{path}", super::API_BASE);
    let response = super::rate_limit_retry::send_with_rate_limit_retry(
        || async {
            if signal.aborted() {
                return Err(Error::Cancelled);
            }
            gloo_net::http::Request::get(&url)
                .credentials(web_sys::RequestCredentials::Omit)
                .abort_signal(Some(signal))
                .send()
                .await
                .map_err(|e| Error::Failed {
                    reason: e.to_string(),
                })
        },
        |response: &gloo_net::http::Response| {
            (response.status(), response.headers().get("Retry-After"))
        },
        |seconds| gloo_timers::future::TimeoutFuture::new(seconds * 1000),
    )
    .await?;
    if !response.ok() {
        let body = response.json::<serde_json::Value>().await.ok();
        return Err(Error::PublicRefusal {
            status: response.status(),
            message: body.as_ref().and_then(super::error_body_message),
        });
    }
    response.json().await.map_err(|e| Error::Failed {
        reason: e.to_string(),
    })
}
