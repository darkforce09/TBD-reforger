//! The vehicle index route: fetch the database, pick a vehicle, and lay the two panes out.
//!
//! **Role:** owns the request for `GET /vehicle-database`, holds the selection and the search
//! text, and arranges the faction-grouped list and the dossier in a split view.
//! **Position:** the `/vehicles` route, behind the authentication gate.
//! **Signals & state:** a `LocalResource` for the vehicle list, plus `selected_id` and `search`
//! signals shared with the two panes; the `AuthStore` comes from context.
//! **Invariants:** the selection starts on the first row and falls back to it whenever the id no
//! longer names a vehicle. The fetch runs on `wasm32` only; natively the resource resolves to
//! `None` and the page renders its failure text.

use super::helpers::vstr;
use super::spec_drawer::dossier;
use super::vehicle_grid::{master_header, vehicle_list};
use crate::v2::core::api::dto::DataEnvelope;
use crate::v2::core::ui::split_pane::GlassSplit;
use leptos::prelude::*;
use serde_json::Value;

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
            crate::v2::core::api::client::api_get::<DataEnvelope<Value>>(store, "/vehicle-database")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
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

/// The split view over a fetched vehicle list.
fn board(rows: Vec<Value>) -> impl IntoView {
    let selected_id = RwSignal::new(rows.first().map(|v| vstr(v, "id")).unwrap_or_default());
    let search = RwSignal::new(String::new());
    let rows_master = rows.clone();
    let rows_detail = rows;

    view! {
        <GlassSplit
            master_width="18rem"
            master_header=master_header(search).into_any()
            master=view! { {move || vehicle_list(selected_id, &search.get(), &rows_master)} }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let v = rows_detail
                        .iter()
                        .find(|r| vstr(r, "id") == id)
                        .cloned()
                        .or_else(|| rows_detail.first().cloned());
                    match v {
                        Some(row) => dossier(row).into_any(),
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
                .into_any()
        />
    }
}
