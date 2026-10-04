//! The page-wide offline signals: the pack's status, its optional-file coverage and its refresh
//! outcome, each mirrored onto the document element.
//!
//! **Role:** holds one signal per status word of [`crate::pack_status`], hands pages a read-only
//! view of each, and publishes a new value by setting the signal and writing its attribute.
//! **Position:** written by the boot registration
//! (`crate::service_worker_registration::register_at_boot`) and the pack download
//! ([`crate::offline_pack::ensure_offline_pack`]); read by the mortar calculator's offline pack
//! line through [`offline_status`], [`offline_optional_files`] and [`offline_pack_refresh`], and
//! by the offline browser gate through the attributes.
//! **Signals & state:** three thread-local `ArcRwSignal`s, one per page lifetime.
//! **Invariants:** the four document-element attributes always equal their signals; a finished
//! download publishes the optional files and the refresh outcome before its final state. The
//! publishers are crate-private and called only by the browser halves, so the native build
//! compiles them for the tests alone; the attribute writes are wasm32 only.

#[cfg(any(target_arch = "wasm32", test))]
use leptos::prelude::Set;
use leptos::prelude::{ArcReadSignal, ArcRwSignal};

#[cfg(target_arch = "wasm32")]
use crate::pack_status::{
    OFFLINE_OPTIONAL_ATTRIBUTE, OFFLINE_PROGRESS_ATTRIBUTE, OFFLINE_REFRESH_ATTRIBUTE,
    OFFLINE_STATE_ATTRIBUTE,
};
use crate::pack_status::{OfflineStatus, OptionalFiles, PackRefresh};

thread_local! {
    static OFFLINE_STATUS: ArcRwSignal<OfflineStatus> = ArcRwSignal::new(OfflineStatus::IDLE);
    static OFFLINE_OPTIONAL_FILES: ArcRwSignal<OptionalFiles> =
        ArcRwSignal::new(OptionalFiles::Unknown);
    static OFFLINE_PACK_REFRESH: ArcRwSignal<PackRefresh> = ArcRwSignal::new(PackRefresh::Unknown);
}

/// The page-wide offline status, for pages that render it: the mortar calculator's offline pack
/// line reads it, and `publish_status` writes it.
pub fn offline_status() -> ArcReadSignal<OfflineStatus> {
    OFFLINE_STATUS.with(|status| status.read_only())
}

/// The page-wide coverage of the optional pack files, for pages that word it: the mortar
/// calculator's offline pack line reads it, and `publish_optional_files` writes it.
pub fn offline_optional_files() -> ArcReadSignal<OptionalFiles> {
    OFFLINE_OPTIONAL_FILES.with(|optional| optional.read_only())
}

/// Whether the last finished download refreshed the saved copy, for pages that word it: the
/// mortar calculator's offline pack line reads it, and `publish_pack_refresh` writes it.
pub fn offline_pack_refresh() -> ArcReadSignal<PackRefresh> {
    OFFLINE_PACK_REFRESH.with(|refresh| refresh.read_only())
}

/// The document element, which carries the four offline attributes.
#[cfg(target_arch = "wasm32")]
fn document_element() -> Option<web_sys::Element> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
}

/// Sets the page-wide refresh outcome and mirrors it onto the document element.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn publish_pack_refresh(refresh: PackRefresh) {
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = document_element() {
        let _ = match refresh.attribute_value() {
            Some(value) => root.set_attribute(OFFLINE_REFRESH_ATTRIBUTE, value),
            None => root.remove_attribute(OFFLINE_REFRESH_ATTRIBUTE),
        };
    }
    OFFLINE_PACK_REFRESH.with(|signal| signal.set(refresh));
}

/// Sets the page-wide optional-file coverage and mirrors it onto the document element.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn publish_optional_files(optional: OptionalFiles) {
    OFFLINE_OPTIONAL_FILES.with(|signal| signal.set(optional));
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = document_element() {
        let _ = match optional.attribute_value() {
            Some(value) => root.set_attribute(OFFLINE_OPTIONAL_ATTRIBUTE, value),
            None => root.remove_attribute(OFFLINE_OPTIONAL_ATTRIBUTE),
        };
    }
}

/// Sets the page-wide status and mirrors it onto the document element.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn publish_status(status: OfflineStatus) {
    OFFLINE_STATUS.with(|signal| signal.set(status));
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = document_element() {
        let _ = root.set_attribute(OFFLINE_STATE_ATTRIBUTE, status.state.attribute_value());
        let _ = root.set_attribute(
            OFFLINE_PROGRESS_ATTRIBUTE,
            &status.progress_percent.to_string(),
        );
    }
}

#[cfg(test)]
#[path = "tests/status_signals.rs"]
mod tests;
