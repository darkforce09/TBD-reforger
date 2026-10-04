//! Saved copies: a cache-backed file read from the server, or from the copy the offline pack saved
//! when the server cannot answer.
//!
//! **Role:** reads one `OfflineTarget` as text and says where the text came from
//! ([`ReadSource`]): the server, or a saved copy — answered by the offline worker (marked with
//! `SAVED_COPY_HEADER`) or, when no worker answers, read from Cache Storage by the page itself.
//! **Position:** the offline pack download reads its listings through `read_text`; the mortar
//! calculator's catalog source reads the catalog list and documents through it, under the
//! targets of [`crate::offline_manifest::catalog_list_target`] and
//! [`crate::offline_manifest::catalog_version_target`], the keys the pack writes.
//! **Signals & state:** none; the browser half reads Cache Storage and the network.
//! **Invariants:** a saved copy answers exactly when
//! [`offline_cache_policy::network_fallback::prefers_saved_copy`] says so — an
//! unreachable server or a `5xx`, never a `4xx`; a saved copy is read from the cache the worker's
//! [`RequestClass::cache_name`](offline_cache_policy::request_classification::RequestClass::cache_name)
//! names for the target's class, under the target's key; the reads carry no credentials; a `429`
//! is waited out and sent again through
//! [`send_with_rate_limit_retry`](frontend_transport::client::rate_limit_retry::send_with_rate_limit_retry)
//! before the answer is judged.

#[cfg(any(target_arch = "wasm32", test))]
use offline_cache_policy::network_fallback::SAVED_COPY_HEADER;
use offline_cache_policy::network_fallback::saved_on_from_date_header;

#[cfg(target_arch = "wasm32")]
use crate::offline_manifest::OfflineTarget;

/// Where a successful read came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadSource {
    /// The server answered.
    Server,
    /// The server could not answer and the saved copy did; `saved_on` is the saved copy's HTTP
    /// `Date`, worded by [`saved_on_from_date_header`], `None` when it carries no readable date.
    SavedCopy {
        /// When the server dated the saved copy, worded for a person.
        saved_on: Option<String>,
    },
}

/// The source of a successful answer whose `SAVED_COPY_HEADER` is `marked` (or not) and whose
/// `Date` header is `date`.
pub fn source_of_answer(marked: bool, date: Option<&str>) -> ReadSource {
    if marked {
        ReadSource::SavedCopy {
            saved_on: date.and_then(saved_on_from_date_header),
        }
    } else {
        ReadSource::Server
    }
}

/// The outcome of [`read_text`].
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRead {
    /// The body, with where it came from.
    Found {
        /// The response body.
        body: String,
        /// Where the body came from.
        source: ReadSource,
    },
    /// The server answered `404` and no saved copy stands in for a refusal.
    NotFound,
}

/// Reads `target` from the server, or from its saved copy when the server cannot answer.
///
/// # Errors
///
/// [`crate::Error::ReadFailed`] with `Request failed (<status>)` for any other refusal, or with
/// the network failure's message, when no saved copy answers.
#[cfg(target_arch = "wasm32")]
pub async fn read_text(target: &OfflineTarget) -> crate::Result<TextRead> {
    browser::read_text(target)
        .await
        .map_err(crate::Error::ReadFailed)
}

/// When the saved copy of `target` was dated by the server, or `None` when it is not cached or
/// carries no readable `Date`.
#[cfg(target_arch = "wasm32")]
pub async fn saved_copy_date(target: &OfflineTarget) -> Option<String> {
    let saved = browser::saved_copy(target).await.ok().flatten()?;
    let date = saved.headers().get("date").ok().flatten()?;
    saved_on_from_date_header(&date)
}

/// The browser half: the credential-free fetch and the Cache Storage read.
#[cfg(target_arch = "wasm32")]
mod browser {
    use offline_cache_policy::cache_names::CacheNames;
    use offline_cache_policy::network_fallback::{NetworkAnswer, prefers_saved_copy};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Cache, RequestCredentials, RequestInit, Response};

    use super::{SAVED_COPY_HEADER, TextRead, source_of_answer};
    use crate::offline_manifest::OfflineTarget;
    use crate::service_worker_registration::current_build_id;
    use frontend_transport::client::rate_limit_retry::send_with_rate_limit_retry;

    fn js_error(error: JsValue) -> String {
        format!("{error:?}")
    }

    /// The credential-free read of `url`, a `429` waited out and sent again by the client's
    /// rate-limit retry before the answer decides between the server and the saved copy.
    async fn fetch(url: &str) -> Result<Response, String> {
        send_with_rate_limit_retry(
            || fetch_once(url),
            |response: &Response| {
                let retry_after = response.headers().get("retry-after").ok().flatten();
                (response.status(), retry_after)
            },
            |seconds| gloo_timers::future::TimeoutFuture::new(seconds * 1000),
        )
        .await
    }

    async fn fetch_once(url: &str) -> Result<Response, String> {
        let window = web_sys::window().ok_or("no window")?;
        let init = RequestInit::new();
        init.set_credentials(RequestCredentials::Omit);
        JsFuture::from(window.fetch_with_str_and_init(url, &init))
            .await
            .map_err(|error| format!("{url}: {}", js_error(error)))?
            .dyn_into::<Response>()
            .map_err(js_error)
    }

    async fn body_text(response: &Response) -> Result<String, String> {
        JsFuture::from(response.text().map_err(js_error)?)
            .await
            .map_err(js_error)?
            .as_string()
            .ok_or_else(|| "the body is not text".to_string())
    }

    /// The response the cache of `target`'s class holds under its key.
    pub(super) async fn saved_copy(target: &OfflineTarget) -> Result<Option<Response>, String> {
        let names = CacheNames::for_build(&current_build_id());
        let Some(name) = target.class.cache_name(&names) else {
            return Ok(None);
        };
        let storage = web_sys::window()
            .ok_or("no window")?
            .caches()
            .map_err(js_error)?;
        let cache = JsFuture::from(storage.open(name))
            .await
            .map_err(js_error)?
            .dyn_into::<Cache>()
            .map_err(js_error)?;
        let found = JsFuture::from(cache.match_with_str(&target.key))
            .await
            .map_err(js_error)?;
        if found.is_undefined() || found.is_null() {
            return Ok(None);
        }
        found.dyn_into::<Response>().map(Some).map_err(js_error)
    }

    pub(super) async fn read_text(target: &OfflineTarget) -> Result<TextRead, String> {
        let fetched = fetch(&target.key).await;
        let answer = match &fetched {
            Ok(response) => NetworkAnswer::Status(response.status()),
            Err(_) => NetworkAnswer::Unreachable,
        };
        if prefers_saved_copy(answer)
            && let Some(saved) = saved_copy(target).await?
        {
            let date = saved.headers().get("date").ok().flatten();
            return Ok(TextRead::Found {
                body: body_text(&saved).await?,
                source: source_of_answer(true, date.as_deref()),
            });
        }
        let response = fetched?;
        if response.status() == 404 {
            return Ok(TextRead::NotFound);
        }
        if !response.ok() {
            return Err(format!("Request failed ({})", response.status()));
        }
        let headers = response.headers();
        let marked = headers.get(SAVED_COPY_HEADER).ok().flatten().is_some();
        let date = headers.get("date").ok().flatten();
        Ok(TextRead::Found {
            body: body_text(&response).await?,
            source: source_of_answer(marked, date.as_deref()),
        })
    }
}

#[cfg(test)]
#[path = "tests/saved_copies.rs"]
mod tests;
