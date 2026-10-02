//! The ballistics catalog route: upload a catalog version, read its validation, see what is stored.
//!
//! **Role:** puts the screen behind the administrator gate and lays out the upload form with its
//! validation report beside the list of stored versions.
//! **Position:** the `/admin/ballistics-catalogs` route, rendered inside the navigation frame.
//! **Signals & state:** owns the upload form's [`UploadDesk`] and the list's reload counter, which
//! a stored upload and the refresh control both bump.
//! **Invariants:** only an accepted upload reloads the list, because only an accepted upload
//! stores a version.
#![allow(dead_code)]

use super::upload_form::{upload_form, UploadDesk};
use super::validation_report::report_panel;
use super::version_list::version_list;
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::{AdminGate, PageHeader};
use leptos::prelude::*;

/// The ballistics catalog screen, behind the administrator gate.
#[component]
pub fn BallisticsCatalogsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <BallisticsCatalogsInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the upload column and the stored versions.
#[component]
fn BallisticsCatalogsInner() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let desk = UploadDesk::new();
    let reload = RwSignal::new(0_u32);
    view! {
        <div data-testid="ballistics-catalogs-page">
            <PageHeader
                title="Ballistics Catalogs"
                subtitle="The weapon and shell values the mortar calculator solves with. Each version is validated against its calibration bundle and never changes once stored."
            />
            <div class="grid gap-6 lg:grid-cols-[minmax(0,26rem)_minmax(0,1fr)]">
                <div class="space-y-4">
                    {upload_form(store, desk, reload)} {report_panel(desk.outcome)}
                </div>
                {version_list(store, reload)}
            </div>
        </div>
    }
}
