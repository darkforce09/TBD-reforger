//! The vehicle index route: fetch the database, pick a vehicle, lay the two panes out, and host the
//! administrator's vehicle form and delete confirmation.
//!
//! **Role:** owns the request for `GET /vehicle-database`, holds the fetched rows, the selection and
//! the search text, arranges the faction-grouped list and the dossier in a split view, and applies
//! each accepted write to the rows in place.
//! **Position:** the `/vehicles` route, behind the authentication gate.
//! **Signals & state:** a `LocalResource` for the vehicle list; `rows`, `selected_id` and `search`
//! signals shared with the two panes; the form's `form_open` and `form_target`, and the delete
//! confirmation's `delete_open` and `delete_target`; an `is_admin` memo over the `AuthStore` from
//! context; the toast queue from context.
//! **Invariants:** the admin memo re-reads the store, so the add, edit and delete actions appear
//! only for a signed-in administrator and never during the session restore. The selection starts on
//! the first row and falls back to it whenever the id no longer names a vehicle. An accepted write
//! changes the one fetched list — a saved row takes its place, a deleted row leaves — so no write
//! costs a second fetch. Every opening of the form writes its target, which re-creates the form
//! with fresh text. The fetch runs on `wasm32` only; natively the resource resolves to `None` and
//! the page renders its failure text.

use super::delete_confirmation::delete_confirmation;
use super::spec_drawer::{dossier, DossierActions};
use super::vehicle_draft::FormTarget;
use super::vehicle_form_dialog::vehicle_form_dialog;
use super::vehicle_grid::{master_header, vehicle_list};
use super::vehicle_rows::{
    place_saved_vehicle, remove_vehicle, selection_after_removal, shown_vehicle,
};
use crate::v2::core::api::dto::vehicles::Vehicle;
use crate::v2::core::api::dto::DataEnvelope;
use crate::v2::core::auth::{has_min_role_authed, Role};
use crate::v2::core::ui::split_pane::GlassSplit;
use leptos::prelude::*;

#[cfg(test)]
#[path = "tests/page.rs"]
mod tests;

/// The vehicle index, behind the authentication gate.
#[component]
pub fn VehicleDatabasePage() -> impl IntoView {
    view! {
        <crate::v2::core::ui::AuthGate>
            <VehiclesInner />
        </crate::v2::core::ui::AuthGate>
    }
}

/// Fetches the vehicle list and renders the board, a loading line, or a failure line.
#[component]
fn VehiclesInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let vehicles = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<DataEnvelope<Vehicle>>(
                store,
                super::vehicle_writes::VEHICLE_DATABASE_PATH,
            )
            .await
            .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Vehicle>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                vehicles
                    .get()
                    .map(|opt| match opt {
                        Some(env) => board(env.data).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load vehicles."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The split view over a fetched vehicle list, with the administrator's dialogs beside it.
fn board(fetched: Vec<Vehicle>) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // Re-read the store on every change: the browse-mode role check treats a signed-out
    // visitor as permitted, so it must never drive the write actions.
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let toasts = crate::v2::core::ui::toast::use_toasts();
    let selected_id = RwSignal::new(fetched.first().map(|v| v.id.clone()).unwrap_or_default());
    let rows = RwSignal::new(fetched);
    let search = RwSignal::new(String::new());
    let form_open = RwSignal::new(false);
    let form_target = RwSignal::new(FormTarget::Create);
    let delete_open = RwSignal::new(false);
    let delete_target = RwSignal::new(None::<Vehicle>);

    let open_form = move |target: FormTarget| {
        form_target.set(target);
        form_open.set(true);
    };
    let on_add = Callback::new(move |()| open_form(FormTarget::Create));
    let actions = DossierActions {
        on_edit: Callback::new(move |vehicle: Vehicle| open_form(FormTarget::Edit(vehicle))),
        on_delete: Callback::new(move |vehicle: Vehicle| {
            delete_target.set(Some(vehicle));
            delete_open.set(true);
        }),
    };
    let on_saved = Callback::new(move |saved: Vehicle| {
        toasts.success(format!("Saved \"{}\"", saved.name));
        selected_id.set(saved.id.clone());
        rows.update(|rows| place_saved_vehicle(rows, saved));
    });
    let on_deleted = Callback::new(move |deleted: Vehicle| {
        toasts.success(format!("Deleted \"{}\"", deleted.name));
        rows.update(|rows| remove_vehicle(rows, &deleted.id));
        let next =
            rows.with_untracked(|rows| selection_after_removal(rows, &selected_id.get_untracked()));
        selected_id.set(next);
    });

    view! {
        <GlassSplit
            master_width="18rem"
            master_header={master_header(search, is_admin, on_add).into_any()}
            master={view! {
                {move || rows.with(|rows| vehicle_list(selected_id, &search.get(), rows))}
            }
                .into_any()}
            detail={view! {
                {move || {
                    let actions = is_admin.get().then_some(actions);
                    let shown = rows
                        .with(|rows| shown_vehicle(rows, &selected_id.get()).cloned());
                    match shown {
                        Some(vehicle) => dossier(vehicle, actions).into_any(),
                        None => view! {
                            <div class="flex h-full items-center justify-center p-8">
                                <p class="font-mono text-sm text-on-surface-variant">
                                    "No vehicles in the database."
                                </p>
                            </div>
                        }
                        .into_any(),
                    }
                }}
            }
                .into_any()}
        />
        {move || vehicle_form_dialog(form_open, form_target.get(), on_saved)}
        {move || {
            delete_target.get().map(|vehicle| delete_confirmation(delete_open, vehicle, on_deleted))
        }}
    }
}
