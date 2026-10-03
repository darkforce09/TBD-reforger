//! The offline pack line: whether the calculator, its catalogs and the Everon map work offline.
//!
//! **Role:** words the page-wide [`OfflineStatus`], [`OptionalFiles`] and [`PackRefresh`] the
//! offline core publishes (and mirrors onto `<html data-offline-state data-offline-progress
//! data-offline-optional data-offline-refresh>`), with a progress bar while downloading.
//! **Position:** under the page header of `/tools/mortar`; the status comes from
//! [`crate::foundation::offline::offline_status`],
//! [`crate::foundation::offline::offline_optional_files`] and
//! [`crate::foundation::offline::offline_pack_refresh`].
//! **Signals & state:** reads the offline status, optional-file and refresh signals; holds
//! nothing.
//! **Invariants:** every [`OfflineState`] has its own sentence; a sentence adds the icon font
//! notice exactly when [`OptionalFiles::Missing`], and the refresh notice (with the saved
//! copy's date) exactly when a `ready` or `incomplete` pack is [`PackRefresh::KeptSavedCopy`];
//! the progress shows only while downloading; the line carries the state's attribute value, so
//! tests and gates read the same state the document element does.

#[cfg(target_arch = "wasm32")]
use crate::foundation::offline::{offline_optional_files, offline_pack_refresh};
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::offline::{OfflineState, OptionalFiles};
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::offline::{OfflineStatus, PackRefresh};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The notice added when the optional icon font is not cached.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const ICON_FONT_MISSING_NOTICE: &str =
    "The icon font is not cached, so icons may show as text offline.";

/// The sentence for `status`, with the [`refresh_notice`] when a usable pack kept its saved copy
/// and [`ICON_FONT_MISSING_NOTICE`] when `optional` is [`OptionalFiles::Missing`].
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn offline_pack_text(
    status: OfflineStatus,
    optional: OptionalFiles,
    refresh: &PackRefresh,
) -> String {
    let mut sentence = state_sentence(status);
    if matches!(status.state, OfflineState::Ready | OfflineState::Incomplete) {
        if let Some(notice) = refresh_notice(refresh) {
            sentence = format!("{sentence} {notice}");
        }
    }
    if optional == OptionalFiles::Missing {
        format!("{sentence} {ICON_FONT_MISSING_NOTICE}")
    } else {
        sentence
    }
}

/// The notice for a pack whose refresh the server could not answer; `None` otherwise.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn refresh_notice(refresh: &PackRefresh) -> Option<String> {
    match refresh {
        PackRefresh::KeptSavedCopy {
            saved_on: Some(date),
        } => Some(format!("Refresh failed, using the saved copy from {date}.")),
        PackRefresh::KeptSavedCopy { saved_on: None } => {
            Some("Refresh failed, using the saved copy (undated).".to_string())
        }
        PackRefresh::Unknown | PackRefresh::Refreshed => None,
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn state_sentence(status: OfflineStatus) -> String {
    match status.state {
        OfflineState::Idle => "Offline copy: not started.".to_string(),
        OfflineState::Downloading => format!(
            "Saving the calculator for offline use… {}%",
            status.progress_percent
        ),
        OfflineState::Ready => "Offline copy ready: the calculator, its catalogs and the Everon \
             map work without a connection."
            .to_string(),
        OfflineState::Incomplete => "Offline copy incomplete: the server lists fewer map files \
             than the offline pack needs."
            .to_string(),
        OfflineState::QuotaShort => {
            "Offline copy not saved: this browser has too little free storage.".to_string()
        }
        OfflineState::Unsupported => "Offline use is not available in this browser.".to_string(),
        OfflineState::Failed => {
            "Offline copy failed: a file could not be downloaded or stored.".to_string()
        }
    }
}

/// The offline pack line.
#[cfg(target_arch = "wasm32")]
pub(crate) fn offline_pack_line(status: Signal<OfflineStatus>) -> impl IntoView {
    let optional = offline_optional_files();
    let refresh = offline_pack_refresh();
    view! {
        <div
            class="flex flex-col gap-1 text-xs text-on-surface-variant"
            data-mortar-offline=move || status.get().state.attribute_value()
        >
            <p>{move || offline_pack_text(status.get(), optional.get(), &refresh.get())}</p>
            {move || {
                let current = status.get();
                (current.state == OfflineState::Downloading)
                    .then(|| {
                        view! {
                            <progress
                                class="h-1 w-48"
                                max="100"
                                prop:value=f64::from(current.progress_percent)
                            ></progress>
                        }
                    })
            }}
        </div>
    }
}

#[cfg(test)]
#[path = "tests/offline_status.rs"]
mod tests;
