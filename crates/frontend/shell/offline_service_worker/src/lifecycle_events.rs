//! The `install` and `activate` handlers.
//!
//! **Role:** at install, stores the app document and the web manifest in this build's shell
//! cache and skips waiting; at activation, deletes every stale offline cache and claims the open
//! pages.
//! **Position:** exported to the loader
//! `crates/frontend/shell/frontend_application/service_worker.js`, which passes each returned
//! promise to `event.waitUntil`.
//! **Signals & state:** writes Cache Storage only.
//! **Invariants:** activation deletes only caches that
//! [`offline_cache_policy::cache_names::CacheNames::is_stale`] reports, so a cache the
//! worker does not own and the current build's caches always survive.

use js_sys::{Array, Promise};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::ExtendableEvent;

use crate::worker_scope::{cache_names, cache_storage, open_cache, scope, settle};

/// The shell entries stored at install: the app document and the web manifest.
const PRECACHED_SHELL_PATHS: [&str; 2] = ["/", "/manifest.webmanifest"];

/// Handles `install`; the loader passes the returned promise to `event.waitUntil`.
#[expect(unreachable_pub, reason = "a #[wasm_bindgen] export is public")]
#[wasm_bindgen]
pub fn on_install(_event: ExtendableEvent) -> Promise {
    future_to_promise(async {
        let shell = open_cache(&cache_names().shell).await?;
        let paths: Array = PRECACHED_SHELL_PATHS
            .iter()
            .copied()
            .map(JsValue::from_str)
            .collect();
        JsFuture::from(shell.add_all_with_str_sequence(&paths)).await?;
        JsFuture::from(scope().skip_waiting()?).await?;
        Ok(JsValue::UNDEFINED)
    })
}

/// Handles `activate`; the loader passes the returned promise to `event.waitUntil`.
#[expect(unreachable_pub, reason = "a #[wasm_bindgen] export is public")]
#[wasm_bindgen]
pub fn on_activate(_event: ExtendableEvent) -> Promise {
    future_to_promise(async {
        let storage = cache_storage()?;
        let existing: Array = settle(storage.keys()).await?;
        let existing: Vec<String> = existing
            .iter()
            .filter_map(|name| name.as_string())
            .collect();
        for stale in cache_names().stale_names(&existing) {
            JsFuture::from(storage.delete(stale)).await?;
        }
        JsFuture::from(scope().clients().claim()).await?;
        Ok(JsValue::UNDEFINED)
    })
}
