//! The `fetch` handler: cache-first, network-first and passthrough over Cache Storage.
//!
//! **Role:** classifies each intercepted request with
//! [`offline_cache_policy::request_classification::classify`] and answers it by the
//! class's strategy, slicing `Range` requests for map assets out of the cached full body.
//! **Position:** exported to the loader
//! `crates/frontend/shell/frontend_application/service_worker.js`, which passes the returned
//! promise to `event.respondWith` for every request.
//! **Signals & state:** reads and writes Cache Storage; cache writes run under
//! `event.waitUntil` so the response is never held back by a write.
//! **Invariants:** only successful responses are stored (`200`, or an opaque icon-font response),
//! the shell document only when it is HTML; responses are stored and returned with every header
//! they carry, so the document's `Cross-Origin-Opener-Policy` and
//! `Cross-Origin-Embedder-Policy` survive offline; a passthrough touches no cache; a
//! network-first request answers from the saved copy exactly when
//! [`offline_cache_policy::network_fallback::prefers_saved_copy`] says so (an
//! unreachable network or a `5xx`, such as a proxy's `502` while the API is down), marked with
//! [`SAVED_COPY_HEADER`], and a `4xx` reaches the page unchanged; a gateway failure with nothing
//! cached reaches the page as it came, and a network failure with nothing cached rejects, which
//! the browser reports as a network error.

use js_sys::Promise;
use offline_cache_policy::network_fallback::{
    NetworkAnswer, SAVED_COPY_HEADER, prefers_saved_copy,
};
use offline_cache_policy::request_classification::{
    CacheStrategy, InterceptedRequest, RequestClass, cache_key, classify,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{
    Blob, Cache, FetchEvent, Headers, Request, RequestMode, Response, ResponseInit, ResponseType,
};

use crate::cached_range_response::answer_from_cached_body;
use crate::worker_scope::{
    cache_names, cached_response, fetch_from_network, open_cache, origin, settle,
};

/// Handles `fetch`; the loader passes the returned promise to `event.respondWith`.
#[expect(unreachable_pub, reason = "a #[wasm_bindgen] export is public")]
#[wasm_bindgen]
pub fn on_fetch(event: FetchEvent) -> Promise {
    future_to_promise(async move { respond(&event).await.map(JsValue::from) })
}

async fn respond(event: &FetchEvent) -> Result<Response, JsValue> {
    let request = event.request();
    let method = request.method();
    let url = request.url();
    let worker_origin = origin();
    let class = classify(&InterceptedRequest {
        method: &method,
        url: &url,
        is_navigation: request.mode() == RequestMode::Navigate,
        worker_origin: &worker_origin,
    });
    let names = cache_names();
    let (Some(cache_name), Some(key)) = (class.cache_name(&names), cache_key(class, &url)) else {
        return fetch_from_network(&request).await;
    };
    let cache = open_cache(cache_name).await?;
    match class.strategy() {
        CacheStrategy::CacheFirst => cache_first(event, &request, class, &cache, &key).await,
        CacheStrategy::NetworkFirst => network_first(event, &request, class, &cache, &key).await,
        CacheStrategy::Passthrough => fetch_from_network(&request).await,
    }
}

async fn cache_first(
    event: &FetchEvent,
    request: &Request,
    class: RequestClass,
    cache: &Cache,
    key: &str,
) -> Result<Response, JsValue> {
    let range = request.headers().get("range")?;
    if let Some(cached) = cached_response(cache, key).await? {
        return match (class.slices_ranges(), range) {
            (true, Some(range)) => answer_from_cached_body(cached, &range).await,
            _ => Ok(cached),
        };
    }
    let response = fetch_from_network(request).await?;
    if range.is_none() {
        store_in_background(event, cache, key, class, &response)?;
    }
    Ok(response)
}

async fn network_first(
    event: &FetchEvent,
    request: &Request,
    class: RequestClass,
    cache: &Cache,
    key: &str,
) -> Result<Response, JsValue> {
    let fetched = fetch_from_network(request).await;
    let answer = match &fetched {
        Ok(response) => NetworkAnswer::Status(response.status()),
        Err(_) => NetworkAnswer::Unreachable,
    };
    if prefers_saved_copy(answer)
        && let Some(saved) = cached_response(cache, key).await?
    {
        return marked_saved_copy(saved).await;
    }
    let response = fetched?;
    store_in_background(event, cache, key, class, &response)?;
    Ok(response)
}

/// The saved copy `saved` with every header it was stored with plus [`SAVED_COPY_HEADER`], so
/// the page can tell that the network failed; the cache entry itself stays unmarked.
async fn marked_saved_copy(saved: Response) -> Result<Response, JsValue> {
    let headers = Headers::new_with_headers(&saved.headers())?;
    headers.set(SAVED_COPY_HEADER, "1")?;
    let body: Blob = settle(saved.blob()?).await?;
    let init = ResponseInit::new();
    init.set_status(saved.status());
    init.set_status_text(&saved.status_text());
    init.set_headers_headers(&headers);
    Response::new_with_opt_blob_and_init(Some(&body), &init)
}

/// Stores a copy of `response` under `key` when it is worth keeping, without delaying the answer;
/// a failed write (such as an exhausted quota) leaves the cache as it was.
fn store_in_background(
    event: &FetchEvent,
    cache: &Cache,
    key: &str,
    class: RequestClass,
    response: &Response,
) -> Result<(), JsValue> {
    if !is_storable(class, response)? {
        return Ok(());
    }
    let write = cache.put_with_str(key, &response.clone()?);
    let settled = future_to_promise(async move {
        let _ = JsFuture::from(write).await;
        Ok(JsValue::UNDEFINED)
    });
    event.wait_until(&settled)
}

fn is_storable(class: RequestClass, response: &Response) -> Result<bool, JsValue> {
    if class == RequestClass::IconFont && response.type_() == ResponseType::Opaque {
        return Ok(true);
    }
    if response.status() != 200 {
        return Ok(false);
    }
    if class != RequestClass::ShellDocument {
        return Ok(true);
    }
    Ok(response
        .headers()
        .get("content-type")?
        .is_some_and(|content_type| content_type.starts_with("text/html")))
}
