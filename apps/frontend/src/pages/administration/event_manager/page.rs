//! The operations calendar route: the month grid, the day panel and the four dialogs over them.
//!
//! **Role:** builds the screen's state, puts it behind the administrator gate, and composes the
//! panels and the access sheet in the order the layout depends on.
//! **Position:** the `/admin/events` route, rendered inside the navigation frame.
//! **Signals & state:** creates the [`Manager`] handle every panel below reads; owns nothing else.
//! **Invariants:** the state is built inside this component, so its signals and its four fetches
//! belong to this owner and are disposed with the route. The detach confirmation is rendered last
//! because it and the edit form share a stacking level, and document order is what puts it on top.

#[cfg(target_arch = "wasm32")]
use super::access::access_sheet;
#[cfg(target_arch = "wasm32")]
use super::confirm_dialogs::{delete_confirm, detach_confirm};
#[cfg(target_arch = "wasm32")]
use super::edit_dialog::edit_dialog;
#[cfg(target_arch = "wasm32")]
use super::event_table::event_table;
#[cfg(target_arch = "wasm32")]
use super::schedule_dialog::schedule_dialog;
#[cfg(target_arch = "wasm32")]
use super::state::Manager;
#[cfg(target_arch = "wasm32")]
use crate::foundation::auth::AdminGate;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The operations calendar, behind the administrator gate.
///
/// Renders nothing of its own: the gate decides whether the screen below it is reachable.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn EventManagerPage() -> impl IntoView {
    view! {
        <AdminGate>
            <EventManagerInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the calendar, the day panel, the four dialogs and the access
/// sheet.
#[cfg(target_arch = "wasm32")]
#[component]
fn EventManagerInner() -> impl IntoView {
    let st = Manager::new();
    view! {
        <div class="mx-auto h-full w-full max-w-5xl">
            {event_table(st)}
            {delete_confirm(st)}
            {schedule_dialog(st)}
            {edit_dialog(st)}
            {access_sheet(st.access)}
            {detach_confirm(st)}
        </div>
    }
}
