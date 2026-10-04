//! The offline core: the service worker registration, the offline pack download and the saved
//! copies it leaves behind.
//!
//! **Role:** registers the offline service worker at boot and, on the first visit of the mortar
//! calculator in a page lifetime, downloads the offline pack — the app shell, the ballistics
//! catalogs, the icon font and the Everon terrain — into the caches the worker answers from,
//! publishing the download's state, progress and whether it refreshed the saved copy; pages
//! read a saved copy back through [`saved_copies`] when the server cannot answer.
//! **Position:** a foundation crate above `frontend_transport` (the rate-limit retry of its
//! reads) and `frontend_api_dtos` (the catalog identifiers of the list it reads); links the pure
//! policy of `offline_cache_policy` (cache names, request classes, the terrain pack list) so the
//! page writes every entry under the cache and key the worker reads. The app's entry point calls
//! `service_worker_registration::register_at_boot` and mounts
//! [`offline_pack::OfflinePackRouteWatcher`] inside the router; pages read [`offline_status`].
//! **Signals & state:** the page-wide status, optional-file and refresh signals of
//! [`status_signals`]; the once-per-page download flag lives in [`offline_pack`].
//! **Invariants:** the four document-element attributes always equal their signals; the pure
//! halves compile on every target and are unit-tested natively, the browser halves are
//! `#[cfg(target_arch = "wasm32")]` inside each file.

pub mod error;
pub mod offline_manifest;
pub mod offline_pack;
#[cfg(target_arch = "wasm32")]
mod pack_download;
pub mod pack_status;
pub mod prelude;
pub mod saved_copies;
pub mod service_worker_registration;
pub mod status_signals;
pub mod storage_quota;

pub use error::{Error, Result};
pub use pack_status::{
    OFFLINE_OPTIONAL_ATTRIBUTE, OFFLINE_PROGRESS_ATTRIBUTE, OFFLINE_REFRESH_ATTRIBUTE,
    OFFLINE_STATE_ATTRIBUTE, OfflineState, OfflineStatus, OptionalFiles, PackRefresh,
};
pub use status_signals::{offline_optional_files, offline_pack_refresh, offline_status};
