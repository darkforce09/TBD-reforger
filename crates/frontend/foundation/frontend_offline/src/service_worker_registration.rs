//! Service worker registration: the build identifier the page registers the offline worker
//! under, and the registration itself.
//!
//! **Role:** derives the [`BuildId`] of the running build from the content hash Trunk puts in
//! the app bundle's file names ([`build_id_from_asset_urls`]) and registers
//! [`BuildId::script_url`] as the origin's service worker at boot (`register_at_boot`).
//! **Position:** `main.rs` calls `register_at_boot` once, before the app mounts;
//! [`crate::offline_pack`] uses `current_build_id` to name the shell cache it writes, which is
//! the cache the worker registered here reads.
//! **Signals & state:** none of its own; a browser without service workers publishes
//! [`crate::pack_status::OfflineState::Unsupported`].
//! **Invariants:** the page and its worker derive every cache name from the same [`BuildId`];
//! a bundle name without a well-formed hash selects [`BuildId::UNVERSIONED`], never a guess.

use offline_cache_policy::cache_names::BuildId;
use url::Url;

/// The file-name prefix Trunk gives the app bundle: `frontend-<hash>.js` and
/// `frontend-<hash>_bg.wasm`.
pub const APP_BUNDLE_PREFIX: &str = "frontend-";

/// The build identifier in the first app bundle URL among `asset_urls`, or
/// [`BuildId::UNVERSIONED`] when none carries one.
pub fn build_id_from_asset_urls(asset_urls: &[String]) -> BuildId {
    asset_urls
        .iter()
        .find_map(|url| bundle_hash(url))
        .unwrap_or_else(|| BuildId::from_script_query(""))
}

fn bundle_hash(url: &str) -> Option<BuildId> {
    let path = match Url::parse(url) {
        Ok(parsed) => parsed.path().to_owned(),
        Err(_) => url.split(['?', '#']).next().unwrap_or(url).to_owned(),
    };
    let file_name = path.rsplit('/').next()?;
    let rest = file_name.strip_prefix(APP_BUNDLE_PREFIX)?;
    let hash = rest
        .strip_suffix("_bg.wasm")
        .or_else(|| rest.strip_suffix(".js"))?;
    if hash.is_empty() || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    BuildId::parse(hash)
}

/// The `href` of every `link` and the `src` of every `script` in the document, as written.
#[cfg(target_arch = "wasm32")]
pub fn document_asset_urls() -> Vec<String> {
    use wasm_bindgen::JsCast;
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Vec::new();
    };
    let Ok(nodes) = document.query_selector_all("link[href], script[src]") else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|position| nodes.item(position))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .filter_map(|element| {
            element
                .get_attribute("href")
                .or_else(|| element.get_attribute("src"))
        })
        .collect()
}

/// The build identifier of the running page.
#[cfg(target_arch = "wasm32")]
pub fn current_build_id() -> BuildId {
    build_id_from_asset_urls(&document_asset_urls())
}

/// Whether the browser offers service workers and cache storage to this page.
#[cfg(target_arch = "wasm32")]
pub fn offline_supported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    let has = |target: &wasm_bindgen::JsValue, name: &str| {
        js_sys::Reflect::get(target, &name.into())
            .map(|value| !value.is_undefined() && !value.is_null())
            .unwrap_or(false)
    };
    window.is_secure_context() && has(&navigator, "serviceWorker") && has(&window, "caches")
}

/// Registers the offline service worker of the running build; a browser without service
/// workers publishes [`crate::pack_status::OfflineState::Unsupported`] instead.
#[cfg(target_arch = "wasm32")]
pub fn register_at_boot() {
    if !offline_supported() {
        crate::status_signals::publish_status(crate::pack_status::OfflineStatus {
            state: crate::pack_status::OfflineState::Unsupported,
            progress_percent: 0,
        });
        return;
    }
    crate::status_signals::publish_status(crate::pack_status::OfflineStatus::IDLE);
    let Some(window) = web_sys::window() else {
        return;
    };
    let script_url = current_build_id().script_url();
    let promise = window.navigator().service_worker().register(&script_url);
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = wasm_bindgen_futures::JsFuture::from(promise).await {
            leptos::logging::warn!("offline service worker registration failed: {error:?}");
        }
    });
}

#[cfg(test)]
#[path = "tests/service_worker_registration.rs"]
mod tests;
