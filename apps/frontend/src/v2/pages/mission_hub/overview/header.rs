//! The mission dossier's header: title, attribution and the armory affordance.
//!
//! **Role:** names the operation, says who wrote it, on what terrain and at what version, and —
//! for whoever may edit it — opens the Edit Armory dialog.
//! **Position:** the top of the `/missions/:id` route, above the glass dossier card.
//! **Signals & state:** none of its own; the armory handle it is given carries every signal the
//! dialog needs.
//! **Invariants:** the edit affordance mirrors the API's own predicate — author or administrator
//! — rather than a role tier, so it is never shown to someone the write would refuse. The mission
//! is snapshotted at click time, so the dialog can only ever carry the mission it was opened on.

use super::armory_editor::ArmoryEditor;
use super::dossier_body::terrain_label;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The dossier header for one mission.
pub(super) fn dossier_header(m: &MissionDetail, ed: ArmoryEditor, editable: bool) -> impl IntoView {
    let version_suffix = m
        .current_version
        .as_ref()
        .map(|v| format!(" — v{}", v.semver))
        .unwrap_or_default();
    let subtitle = format!(
        "by {} — Terrain: {}{}",
        m.author_name,
        terrain_label(&m.terrain),
        version_suffix
    );
    // Snapshot for the dialog, taken at click time: it is loaded from the mission it was opened
    // on, so nothing it sends can be carrying another mission's data.
    let snapshot = m.clone();
    view! {
            <header class="mb-8 flex flex-wrap items-start justify-between gap-4">
                <div class="min-w-0">
                    <h1 class="mb-2 text-3xl font-bold text-on-surface">{m.title.clone()}</h1>
                    <p class="max-w-3xl text-on-surface-variant">{subtitle}</p>
                </div>
                {editable
                    .then(move || {
                        view! {
                            <button
                                type="button"
                                on:click=move |_| ed.open_for(&snapshot)
                                class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="inventory_2" class="text-base" />
                                "Edit Armory"
                            </button>
                        }
                    })}
            </header>
    }
}
