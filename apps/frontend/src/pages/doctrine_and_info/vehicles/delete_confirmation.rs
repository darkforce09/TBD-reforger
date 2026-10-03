//! The confirmation an administrator answers before a vehicle leaves the database.
//!
//! **Role:** names the vehicle about to be deleted, sends `DELETE /vehicle-database/{id}` once the
//! administrator confirms, and shows why a delete was refused.
//! **Position:** a [`Dialog`] over the vehicle index, opened from the dossier's Delete action; the
//! page re-creates it for every vehicle it is opened on.
//! **Signals & state:** owns the refusal sentence and the busy flag. Closes the page's `open` flag
//! and hands the deleted row to `on_deleted` when the backend accepts; reads the `AuthStore` from
//! context.
//! **Invariants:** nothing is sent before the confirm button is pressed, and a delete is in flight
//! at most once. The wording matches the backend's delete, which takes the row out of every list
//! and route and offers no way back from any page. The send runs on `wasm32` only, as does the
//! confirmation.

#[cfg(target_arch = "wasm32")]
use super::vehicle_writes::VehicleRequest;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::vehicles::Vehicle;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::Dialog;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The confirmation's title.
#[cfg(target_arch = "wasm32")]
pub(super) const DELETE_VEHICLE_TITLE: &str = "Delete this vehicle?";
/// What the delete does, as the confirmation states it.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const DELETE_VEHICLE_DESCRIPTION: &str = "The vehicle leaves the vehicle database for \
     every member. No page can bring it back.";
/// The sentence shown when a refused delete carries no sentence of its own.
#[cfg(target_arch = "wasm32")]
const DELETE_FALLBACK: &str = "The vehicle could not be deleted.";

/// The delete confirmation for `vehicle`, open while `open` holds; `on_deleted` receives the
/// vehicle once the backend has deleted it.
#[cfg(target_arch = "wasm32")]
pub(super) fn delete_confirmation(
    open: RwSignal<bool>,
    vehicle: Vehicle,
    on_deleted: Callback<Vehicle>,
) -> impl IntoView {
    let store = expect_context::<crate::foundation::auth::AuthStore>();
    let refusal = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let name = vehicle.name.clone();
    let vehicle = StoredValue::new(vehicle);

    let confirm = move || {
        if busy.get_untracked() {
            return;
        }
        refusal.set(None);
        busy.set(true);
        let request = vehicle.with_value(|vehicle| VehicleRequest::delete(&vehicle.id));
        leptos::task::spawn_local(async move {
            let answer = super::vehicle_writes::send_vehicle_request(store, request).await;
            busy.set(false);
            match answer {
                Ok(_) => {
                    open.set(false);
                    on_deleted.run(vehicle.get_value());
                }
                Err(refused) => refusal.set(Some(super::write_refusal::refusal_sentence(
                    &refused,
                    DELETE_FALLBACK,
                ))),
            }
        });
    };

    view! {
        <Dialog
            open=open
            title=DELETE_VEHICLE_TITLE
            description=DELETE_VEHICLE_DESCRIPTION
        >
            <p class="mb-4 truncate text-sm font-bold text-on-surface">{name.clone()}</p>
            {move || {
                refusal
                    .get()
                    .map(|sentence| {
                        view! {
                            <p role="alert" class="mb-4 font-mono text-sm text-error-alert">
                                {sentence}
                            </p>
                        }
                    })
            }}
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| open.set(false)
                    class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=move |_| confirm()
                    prop:disabled=move || busy.get()
                    class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                >
                    {move || if busy.get() { "Deleting…" } else { "Delete vehicle" }}
                </button>
            </div>
        </Dialog>
    }
}
