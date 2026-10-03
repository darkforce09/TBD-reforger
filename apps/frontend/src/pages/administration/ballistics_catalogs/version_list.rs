//! The stored catalog versions: every version the API holds, newest first within each catalog.
//!
//! **Role:** fetches `GET /api/v1/ballistics-catalogs` and renders its rows, with the loading,
//! failed and empty states and a manual refresh.
//! **Position:** the right column of the `/admin/ballistics-catalogs` route; refetches whenever
//! the upload form bumps the reload counter.
//! **Signals & state:** the fetch is a local resource keyed on the reload counter.
//! **Invariants:** a failed fetch shows its reason and a retry, never an empty list; stored
//! versions are immutable, so a row never changes, it only appears. The fetch runs in the browser
//! build only, as does the list.

#[cfg(target_arch = "wasm32")]
use super::view_model::{version_count_label, version_rows, CatalogVersionRow};
#[cfg(target_arch = "wasm32")]
use crate::foundation::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::client::{api_error_message, ApiErr};
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::ballistics_catalogs::BallisticsCatalogList;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The list panel with its header, count and refresh control.
#[cfg(target_arch = "wasm32")]
pub(super) fn version_list(store: AuthStore, reload: RwSignal<u32>) -> impl IntoView {
    let listing = LocalResource::new(move || {
        let _generation = reload.get();
        fetch_versions(store)
    });
    let refresh = move |_| reload.update(|generation| *generation += 1);
    view! {
        <section
            class="min-w-0 rounded-xl border border-white/10 bg-white/[0.02]"
            data-testid="ballistics-catalog-versions"
        >
            <div class="flex items-center justify-between gap-2 border-b border-outline-variant/30 px-5 py-3">
                <h2 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                    "Stored versions"
                </h2>
                <button
                    type="button"
                    class="inline-flex items-center gap-1 rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    on:click=refresh
                >
                    <MaterialIcon name="refresh" class="text-[16px]" />
                    "Refresh"
                </button>
            </div>
            {move || match listing.get() {
                None => {
                    view! {
                        <p class="px-5 py-6 text-on-surface-variant">"Loading catalog versions…"</p>
                    }
                        .into_any()
                }
                Some(Err(err)) => {
                    view! {
                        <div class="space-y-2 px-5 py-6">
                            <p class="text-error">
                                {api_error_message(&err, "The catalog versions could not be loaded")}
                            </p>
                            <button
                                type="button"
                                class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                                on:click=refresh
                            >
                                "Retry"
                            </button>
                        </div>
                    }
                        .into_any()
                }
                Some(Ok(list)) => version_table(version_rows(&list)).into_any(),
            }}
        </section>
    }
}

/// Load the list: the browser build asks the API; a native build has no API to ask.
#[cfg(target_arch = "wasm32")]
async fn fetch_versions(store: AuthStore) -> Result<BallisticsCatalogList, ApiErr> {
    {
        crate::foundation::transport::client::api_get::<BallisticsCatalogList>(
            store,
            super::view_model::BALLISTICS_CATALOGS_PATH,
        )
        .await
    }
}

/// The rows, or the empty copy when nothing is stored.
#[cfg(target_arch = "wasm32")]
fn version_table(rows: Vec<CatalogVersionRow>) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <p class="px-5 py-6 text-on-surface-variant">
                "No catalog versions are stored yet. Upload the first one with its calibration bundle."
            </p>
        }
        .into_any();
    }
    let count = version_count_label(&rows);
    view! {
        <p class="px-5 pt-3 font-mono text-code-md text-on-surface-variant tabular-nums">{count}</p>
        <div class="overflow-x-auto">
            <table class="w-full text-label-md">
                <thead class="text-label-sm text-on-surface-variant uppercase">
                    <tr>
                        <th class="px-5 py-2 text-left font-medium">"Catalog"</th>
                        <th class="px-3 py-2 text-right font-medium">"Version"</th>
                        <th class="px-3 py-2 text-left font-medium">"Game build"</th>
                        <th class="px-3 py-2 text-left font-medium">"SHA-256"</th>
                        <th class="px-5 py-2 text-left font-medium">"Uploaded"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-white/5">
                    {rows.into_iter().map(version_row).collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

/// One stored version.
#[cfg(target_arch = "wasm32")]
fn version_row(row: CatalogVersionRow) -> impl IntoView {
    let row_catalog_id = row.catalog_id.clone();
    view! {
        <tr data-catalog-id=row_catalog_id data-catalog-version=row.catalog_version>
            <td class="px-5 py-2">
                <span class="block text-on-surface">{row.title}</span>
                <span class="block font-mono text-code-md text-outline">{row.catalog_id}</span>
            </td>
            <td class="px-3 py-2 text-right font-mono text-code-md text-on-surface tabular-nums">
                {row.catalog_version}
                {row
                    .latest
                    .then(|| {
                        view! {
                            <span class="ml-2 rounded border border-primary/40 bg-primary/10 px-1.5 text-label-sm text-primary">
                                "latest"
                            </span>
                        }
                    })}
            </td>
            <td class="px-3 py-2 font-mono text-code-md text-on-surface-variant">{row.game_build}</td>
            <td class="px-3 py-2 font-mono text-code-md text-on-surface-variant" title=row.catalog_sha256>
                {row.short_sha}
            </td>
            <td class="px-5 py-2 font-mono text-code-md text-on-surface-variant">
                {row.uploaded_label}
            </td>
        </tr>
    }
}
