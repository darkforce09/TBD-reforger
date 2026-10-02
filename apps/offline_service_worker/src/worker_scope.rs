//! The worker's global scope and the Cache Storage helpers every handler shares.
//!
//! **Role:** reaches the `ServiceWorkerGlobalScope`, derives this build's cache names from the
//! script URL, and wraps the promise-returning Cache Storage and fetch calls as futures.
//! **Position:** used by `lifecycle_events` and `fetch_handling`; wraps `web-sys`.
//! **Signals & state:** none; every call reads the live global scope.
//! **Invariants:** cache names always come from
//! [`offline_cache_policy::cache_names::CacheNames::for_build`] over the script URL's
//! query, the same derivation the page uses.

use js_sys::Promise;
use offline_cache_policy::cache_names::{BuildId, CacheNames};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, CacheStorage, Request, Response, ServiceWorkerGlobalScope};

/// The running worker's global scope.
pub(crate) fn scope() -> ServiceWorkerGlobalScope {
    js_sys::global().unchecked_into()
}

/// This build's cache names, from the `build` query of the worker's script URL.
pub(crate) fn cache_names() -> CacheNames {
    CacheNames::for_build(&BuildId::from_script_query(&scope().location().search()))
}

/// The worker's origin, such as `https://tbd.example`.
pub(crate) fn origin() -> String {
    scope().origin()
}

/// The worker's Cache Storage.
pub(crate) fn cache_storage() -> Result<CacheStorage, JsValue> {
    scope().caches()
}

/// Awaits `promise` and casts its value to `T`.
pub(crate) async fn settle<T: JsCast>(promise: Promise) -> Result<T, JsValue> {
    JsFuture::from(promise).await?.dyn_into::<T>()
}

/// Opens (creating when absent) the cache `name`.
pub(crate) async fn open_cache(name: &str) -> Result<Cache, JsValue> {
    settle(cache_storage()?.open(name)).await
}

/// The response stored under `key` in `cache`, if any.
pub(crate) async fn cached_response(cache: &Cache, key: &str) -> Result<Option<Response>, JsValue> {
    let found = JsFuture::from(cache.match_with_str(key)).await?;
    if found.is_undefined() {
        Ok(None)
    } else {
        found.dyn_into::<Response>().map(Some)
    }
}

/// Fetches `request` from the network.
pub(crate) async fn fetch_from_network(request: &Request) -> Result<Response, JsValue> {
    settle(scope().fetch_with_request(request)).await
}
