//! The offline core: the service worker registration, the offline pack download and the saved
//! copies it leaves behind.
//!
//! **Role:** registers the offline service worker at boot and, on the first visit of the mortar
//! calculator in a page lifetime, downloads the offline pack — the app shell, the ballistics
//! catalogs, the icon font and the Everon terrain — into the caches the worker answers from,
//! publishing the download's state, progress and whether it refreshed the saved copy; pages
//! read a saved copy back through [`saved_copies`] when the server cannot answer.
//! **Position:** under `core`; links the pure policy of `website_offline_service_worker` (cache
//! names, request classes, the terrain pack list) so the page writes every entry under the cache
//! and key the worker reads. `main.rs` calls
//! [`service_worker_registration::register_at_boot`] and mounts
//! [`offline_pack::OfflinePackRouteWatcher`] inside the router; pages read [`offline_status`].
//! **Signals & state:** one page-wide [`OfflineStatus`] signal, mirrored onto the document
//! element as [`OFFLINE_STATE_ATTRIBUTE`] and [`OFFLINE_PROGRESS_ATTRIBUTE`], one page-wide
//! [`OptionalFiles`] signal, mirrored as [`OFFLINE_OPTIONAL_ATTRIBUTE`], and one page-wide
//! [`PackRefresh`] signal, mirrored as [`OFFLINE_REFRESH_ATTRIBUTE`]; the once-per-page
//! download flag lives in [`offline_pack`].
//! **Invariants:** the four DOM attributes always equal their signals; the state is `ready` only
//! when the pack lists every essential file and every essential file is cached, and an
//! incomplete pack or an essential file missing from the cache is never `ready`; a refresh the
//! server could not answer keeps a complete pack `ready` ([`PackRefresh::KeptSavedCopy`]) while
//! every essential file is still cached; the optional files (the
//! cross-origin icon font) never decide the state, only [`OptionalFiles`], which a finished
//! download publishes before its final state; the pure halves compile natively and are
//! unit-tested, the browser halves are `#[cfg(target_arch = "wasm32")]` inside each file.

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod offline_manifest;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod offline_pack;
#[cfg(target_arch = "wasm32")]
mod pack_download;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod saved_copies;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod service_worker_registration;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod storage_quota;

use leptos::prelude::{ArcReadSignal, ArcRwSignal, Set};

/// The document-element attribute that carries [`OfflineState::attribute_value`].
pub const OFFLINE_STATE_ATTRIBUTE: &str = "data-offline-state";

/// The document-element attribute that carries [`OfflineStatus::progress_percent`], 0 to 100.
pub const OFFLINE_PROGRESS_ATTRIBUTE: &str = "data-offline-progress";

/// The document-element attribute that carries [`OptionalFiles::attribute_value`]; absent while
/// no download has finished.
pub const OFFLINE_OPTIONAL_ATTRIBUTE: &str = "data-offline-optional";

/// The document-element attribute that carries [`PackRefresh::attribute_value`]; absent while
/// no download has finished.
pub const OFFLINE_REFRESH_ATTRIBUTE: &str = "data-offline-refresh";

/// Where the offline pack stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineState {
    /// Nothing has been attempted in this page lifetime.
    Idle,
    /// The pack is downloading; the progress is meaningful.
    Downloading,
    /// Every essential file of a complete pack is cached; [`OptionalFiles`] says whether the
    /// optional ones are too.
    Ready,
    /// Every listed file is cached, but the server lists fewer files than the pack needs (no or
    /// a partial map tile index).
    Incomplete,
    /// The origin's free storage is smaller than the files still to download.
    QuotaShort,
    /// The browser offers no service worker or no cache storage (or the page is not a secure
    /// context).
    Unsupported,
    /// An essential file or the pack list could not be fetched or stored.
    Failed,
}

impl OfflineState {
    /// The value of [`OFFLINE_STATE_ATTRIBUTE`].
    pub fn attribute_value(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Downloading => "downloading",
            Self::Ready => "ready",
            Self::Incomplete => "incomplete",
            Self::QuotaShort => "quota-short",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
        }
    }
}

/// The offline pack's state with its download progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfflineStatus {
    /// Where the pack stands.
    pub state: OfflineState,
    /// Share of the pack cached, 0 to 100; 100 only once every listed file is cached.
    pub progress_percent: u8,
}

impl OfflineStatus {
    /// The status before anything is attempted.
    pub const IDLE: Self = Self {
        state: OfflineState::Idle,
        progress_percent: 0,
    };
}

/// Whether the optional files of the offline pack — the cross-origin Material Symbols icon font's
/// stylesheet and font files — are cached. Without them the calculator works offline, but its
/// icons may show as their ligature text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalFiles {
    /// No download has finished in this page lifetime.
    Unknown,
    /// Every optional file is cached.
    Complete,
    /// An optional file could not be listed, fetched or stored.
    Missing,
}

impl OptionalFiles {
    /// The value of [`OFFLINE_OPTIONAL_ATTRIBUTE`]; `None` removes the attribute.
    pub fn attribute_value(self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Complete => Some("complete"),
            Self::Missing => Some("missing"),
        }
    }
}

/// Whether the last finished download refreshed the saved copy from the server or kept it
/// because the server could not answer (unreachable, or a gateway or server failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRefresh {
    /// No download has finished in this page lifetime.
    Unknown,
    /// The server answered every file the download read.
    Refreshed,
    /// The server could not answer and the saved copy stays in use; `saved_on` is when the server
    /// dated the saved catalog list (see
    /// [`saved_copies::saved_on_from_date_header`]), `None` when it carried no readable date.
    KeptSavedCopy {
        /// The saved catalog list's date, worded for a person.
        saved_on: Option<String>,
    },
}

impl PackRefresh {
    /// The value of [`OFFLINE_REFRESH_ATTRIBUTE`]; `None` removes the attribute.
    pub fn attribute_value(&self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Refreshed => Some("refreshed"),
            Self::KeptSavedCopy { .. } => Some("kept-saved-copy"),
        }
    }
}

thread_local! {
    static OFFLINE_STATUS: ArcRwSignal<OfflineStatus> = ArcRwSignal::new(OfflineStatus::IDLE);
    static OFFLINE_OPTIONAL_FILES: ArcRwSignal<OptionalFiles> =
        ArcRwSignal::new(OptionalFiles::Unknown);
    static OFFLINE_PACK_REFRESH: ArcRwSignal<PackRefresh> = ArcRwSignal::new(PackRefresh::Unknown);
}

/// The page-wide offline status, for pages that render it: the mortar calculator's offline pack
/// line reads it, and [`publish_status`] writes it.
pub fn offline_status() -> ArcReadSignal<OfflineStatus> {
    OFFLINE_STATUS.with(|status| status.read_only())
}

/// The page-wide coverage of the optional pack files, for pages that word it: the mortar
/// calculator's offline pack line reads it, and [`publish_optional_files`] writes it.
pub fn offline_optional_files() -> ArcReadSignal<OptionalFiles> {
    OFFLINE_OPTIONAL_FILES.with(|optional| optional.read_only())
}

/// Whether the last finished download refreshed the saved copy, for pages that word it: the
/// mortar calculator's offline pack line reads it, and [`publish_pack_refresh`] writes it.
pub fn offline_pack_refresh() -> ArcReadSignal<PackRefresh> {
    OFFLINE_PACK_REFRESH.with(|refresh| refresh.read_only())
}

/// Sets the page-wide refresh outcome and mirrors it onto the document element.
pub(crate) fn publish_pack_refresh(refresh: PackRefresh) {
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = match refresh.attribute_value() {
            Some(value) => root.set_attribute(OFFLINE_REFRESH_ATTRIBUTE, value),
            None => root.remove_attribute(OFFLINE_REFRESH_ATTRIBUTE),
        };
    }
    OFFLINE_PACK_REFRESH.with(|signal| signal.set(refresh));
}

/// Sets the page-wide optional-file coverage and mirrors it onto the document element.
pub(crate) fn publish_optional_files(optional: OptionalFiles) {
    OFFLINE_OPTIONAL_FILES.with(|signal| signal.set(optional));
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = match optional.attribute_value() {
            Some(value) => root.set_attribute(OFFLINE_OPTIONAL_ATTRIBUTE, value),
            None => root.remove_attribute(OFFLINE_OPTIONAL_ATTRIBUTE),
        };
    }
}

/// Sets the page-wide status and mirrors it onto the document element.
pub(crate) fn publish_status(status: OfflineStatus) {
    OFFLINE_STATUS.with(|signal| signal.set(status));
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = root.set_attribute(OFFLINE_STATE_ATTRIBUTE, status.state.attribute_value());
        let _ = root.set_attribute(
            OFFLINE_PROGRESS_ATTRIBUTE,
            &status.progress_percent.to_string(),
        );
    }
}

#[cfg(test)]
#[path = "tests/offline_status.rs"]
mod tests;
