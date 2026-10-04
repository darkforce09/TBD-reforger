//! The save area's gate: the session gate while the platform answers, a needs-a-connection notice
//! while the page solves from the copy saved on this device.
//!
//! **Role:** decides from the catalog's [`CatalogOrigin`] whether the save area goes behind
//! `AuthGate` or is replaced by `NEEDS_CONNECTION_TEXT` (`save_area_access`), and renders
//! that choice (`MortarSaveSection`).
//! **Position:** mounted by the mortar page under the solution panel; wraps
//! `super::save_area::MortarSaveArea`.
//! **Signals & state:** reads the page's catalog origin signal through a memo, so the subtree
//! re-renders only when the decision changes.
//! **Invariants:** a page reading the offline copy never offers the sign-in call to action (a
//! session refresh cannot succeed without the platform, and saving and the saved list need it);
//! a page reading from the server, or not yet read, keeps the session gate unchanged.
//!
//! [`CatalogOrigin`]: crate::mortar::catalog_source::CatalogOrigin

#[cfg(target_arch = "wasm32")]
use super::save_area::MortarSaveArea;
#[cfg(target_arch = "wasm32")]
use super::save_area::RestoreTargets;
#[cfg(any(target_arch = "wasm32", test))]
use crate::mortar::catalog_source::CatalogOrigin;
#[cfg(target_arch = "wasm32")]
use crate::mortar::solution::SolveOutcome;
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthGate;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The notice shown in place of the save area while the page reads the offline copy.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const NEEDS_CONNECTION_TEXT: &str = "Saving fire missions and the saved fire missions \
     need a connection to the platform; the calculator above keeps working offline.";

/// What the save area shows.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveAreaAccess {
    /// The platform answers (or has not answered yet): the save area behind the session gate.
    SessionGate,
    /// The page reads the copy saved on this device: the needs-a-connection notice.
    NeedsConnection,
}

/// The save area's access for a catalog read from `origin` (`None` before any catalog loads).
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn save_area_access(origin: Option<&CatalogOrigin>) -> SaveAreaAccess {
    match origin {
        Some(CatalogOrigin::OfflineCopy { .. }) => SaveAreaAccess::NeedsConnection,
        Some(CatalogOrigin::Network) | None => SaveAreaAccess::SessionGate,
    }
}

/// The save area, or the needs-a-connection notice while the page reads the offline copy.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn MortarSaveSection(
    /// Where the page's catalog was read from; `None` before any catalog loads.
    origin: RwSignal<Option<CatalogOrigin>>,
    /// The page's draft signals a restored saved fire mission writes.
    restore_into: RestoreTargets,
    /// The page's last solve outcome, which the save button files.
    outcome: RwSignal<Option<SolveOutcome>>,
) -> impl IntoView {
    let access = Memo::new(move |_| origin.with(|o| save_area_access(o.as_ref())));
    move || match access.get() {
        SaveAreaAccess::NeedsConnection => view! {
            <div class="rounded-xl p-6 glass" data-mortar-save-area="needs-connection">
                <p class="text-sm text-on-surface-variant">{NEEDS_CONNECTION_TEXT}</p>
            </div>
        }
        .into_any(),
        SaveAreaAccess::SessionGate => view! {
            <AuthGate>
                <MortarSaveArea restore_into=restore_into outcome=outcome />
            </AuthGate>
        }
        .into_any(),
    }
}
