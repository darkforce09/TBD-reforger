//! The saved-fire-missions list of the selected event.
//!
//! **Role:** renders every fire mission stored against the selected event, newest first, each a
//! button that loads it back into the inputs, and words each row.
//! **Position:** inside the save area (`super::save_area`), under the event picker and the save
//! button.
//! **Signals & state:** reads the saved-rows resource and the event signal.
//! **Invariants:** "not fetched yet", "fetch failed" and "a batch of the previously selected
//! event" are three different states and never render as an empty list; a legacy row and a
//! catalog-model row are both listed, each with the figures it carries.

#[cfg(target_arch = "wasm32")]
use super::SavedFor;
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::dto::SavedFire;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The two lines of one listed row: the positions, then the figures.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn saved_row_lines(row: &SavedFire) -> (String, String) {
    match (&row.weapon_id, &row.shell_id) {
        (Some(weapon), Some(shell)) => {
            let guns = row.guns.len();
            let plural = if guns == 1 { "gun" } else { "guns" };
            let charge = row.charge_rings.map_or_else(
                || "recommended charge".to_string(),
                |r| format!("charge {r}"),
            );
            (
                format!("{guns} {plural} → {}", row.target_grid),
                format!("{weapon} · {shell} · {charge}"),
            )
        }
        _ => (
            format!("{} → {}", row.fp_grid, row.target_grid),
            format!(
                "{} · {} m · {:.1}° · {} mils",
                row.weapon_system, row.distance_m, row.azimuth_deg, row.elevation_mils
            ),
        ),
    }
}

/// The saved list over `saved` for the selected `event_id`; a click hands the row to `load_row`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn saved_list(
    saved: LocalResource<Option<SavedFor>>,
    event_id: RwSignal<Option<String>>,
    load_row: impl Fn(SavedFire) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let loading = || view! { <p class="text-xs text-on-surface-variant">"Loading…"</p> }.into_any();
    view! {
        <div class="flex max-h-72 flex-col gap-2 overflow-hidden" data-mortar-saved="">
            <h2 class="text-sm font-semibold text-primary">"Saved Fire Missions"</h2>
            {move || {
                // The outer `Option` is "not resolved yet", the inner one "the fetch failed".
                let batch = saved.get();
                let stale = matches!(&batch, Some(Some(b)) if b.event_id != event_id.get());
                match batch {
                    _ if stale => loading(),
                    None => loading(),
                    Some(None) => view! {
                        <p class="text-xs text-error">"Could not load saved fire missions."</p>
                    }
                    .into_any(),
                    Some(Some(SavedFor { rows, .. })) if rows.is_empty() => view! {
                        <p class="text-xs text-on-surface-variant">
                            {move || {
                                if event_id.get().is_some() {
                                    "Nothing saved on this event yet."
                                } else {
                                    "Pick an event to save and reload solutions."
                                }
                            }}
                        </p>
                    }
                    .into_any(),
                    Some(Some(SavedFor { rows, .. })) => view! {
                        <ul class="flex min-h-0 flex-col gap-1 overflow-y-auto font-mono text-xs">
                            {rows
                                .into_iter()
                                .rev()
                                .map(|row| {
                                    let (positions, figures) = saved_row_lines(&row);
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                on:click=move |_| load_row(row.clone())
                                                class="w-full rounded px-2 py-1 text-left hover:bg-surface-variant/60"
                                            >
                                                <span class="text-on-surface">{positions}</span>
                                                <span class="block text-on-surface-variant">{figures}</span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                    .into_any(),
                }
            }}
        </div>
    }
}
