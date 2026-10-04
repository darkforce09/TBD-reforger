//! The detail half of the vehicle index: one vehicle's identification dossier.
//!
//! **Role:** renders the selected vehicle — its photograph or a placeholder, the armour,
//! amphibious and faction chips, its primary threat note, a small telemetry grid, and, for an
//! administrator, the Edit and Delete actions.
//! **Position:** the detail pane of the vehicle index's split view.
//! **Signals & state:** none — it is handed an owned row and, for an administrator, the two
//! callbacks the page answers the actions with.
//! **Invariants:** every optional field may be empty; an empty or unsafe image URL shows the
//! placeholder icon ([`profile_image_src`] admits only what
//! [`frontend_ui::safe_url::safe_image_src`] admits), an empty amphibious value drops
//! the chip, and an empty threat shows a fixed line instead. The actions render only when the
//! page hands them over, which it does for a signed-in administrator alone.

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::vehicles::Vehicle;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_ui::safe_url::safe_image_src;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The neutral chip, used for the armour class.
#[cfg(any(target_arch = "wasm32", test))]
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
/// The warning chip, used for an amphibious vehicle.
#[cfg(any(target_arch = "wasm32", test))]
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";
/// The success chip, used for a vehicle that is not amphibious.
#[cfg(any(target_arch = "wasm32", test))]
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
/// The primary chip, used for the faction.
#[cfg(target_arch = "wasm32")]
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";
/// The frosted button the administrator's actions share over the photograph.
#[cfg(target_arch = "wasm32")]
const ACTION_BUTTON: &str = "flex items-center gap-1.5 rounded-lg border border-white/15 bg-black/40 px-3 py-1.5 text-label-md text-on-surface backdrop-blur-md transition-colors hover:bg-black/60";
/// The destructive variant of [`ACTION_BUTTON`].
#[cfg(target_arch = "wasm32")]
const DELETE_BUTTON: &str = "flex items-center gap-1.5 rounded-lg border border-error-alert/30 bg-black/40 px-3 py-1.5 text-label-md text-error-alert backdrop-blur-md transition-colors hover:bg-error-alert/20";

/// The administrator's answers to the dossier's actions, each handed the row on screen.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct DossierActions {
    /// Opens the vehicle form on the row.
    pub on_edit: Callback<Vehicle>,
    /// Opens the delete confirmation for the row.
    pub on_delete: Callback<Vehicle>,
}

/// The chip class for an amphibious value.
///
/// A recognised yes reads as a warning, a recognised no as a success, and anything else —
/// including an unrecognised spelling — as neutral.
#[cfg(any(target_arch = "wasm32", test))]
fn amphib_badge(amphibious: &str) -> &'static str {
    match amphibious.trim().to_ascii_lowercase().as_str() {
        "yes" | "y" | "true" => BADGE_WARNING,
        "no" | "n" | "false" => BADGE_SUCCESS,
        _ => BADGE_NEUTRAL,
    }
}

/// The source the dossier's photograph loads from, or `None` for the placeholder: an empty URL and
/// every URL the content URL policy refuses both show the placeholder.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn profile_image_src(vehicle: &Vehicle) -> Option<&str> {
    safe_image_src(&vehicle.profile_image_url)
}

/// The identification dossier for one vehicle; `actions` is `Some` for an administrator only.
#[cfg(target_arch = "wasm32")]
pub(super) fn dossier(v: Vehicle, actions: Option<DossierActions>) -> impl IntoView {
    let hero = match profile_image_src(&v) {
        Some(src) => {
            view! { <img src={src.to_owned()} alt="" class="h-full w-full object-cover" /> }
                .into_any()
        }
        None => view! {
            <div class="flex h-full w-full items-center justify-center bg-surface-container-low">
                <MaterialIcon name="directions_car" class="text-7xl text-outline" />
            </div>
        }
        .into_any(),
    };
    let toolbar = actions.map(|actions| admin_toolbar(&v, actions));
    let Vehicle {
        name,
        faction,
        armor_type: armor,
        amphibious: amphib,
        primary_threat: threat,
        ..
    } = v;

    let amphib_label = if amphib.is_empty() {
        "—".to_string()
    } else {
        amphib.clone()
    };
    let threat_body = if threat.is_empty() {
        "No primary threat recorded.".to_string()
    } else {
        threat
    };

    view! {
        <div>
            <div class="relative h-72 w-full overflow-hidden">
                {hero}
                <div class="absolute inset-0 bg-gradient-to-t from-surface-dim to-transparent"></div>
                {toolbar}
                <div class="absolute right-8 bottom-6 left-8">
                    <div class="mb-3 flex flex-wrap items-center gap-2">
                        <span class=BADGE_NEUTRAL>"ARMOR: "{armor.clone()}</span>
                        {if !amphib.is_empty() {
                            view! {
                                <span class={amphib_badge(&amphib)}>"AMPHIB: "{amphib.clone()}</span>
                            }
                                .into_any()
                        } else {
                            ().into_any()
                        }}
                        <span class=BADGE_PRIMARY>{faction.clone()}</span>
                    </div>
                    <h1 class="text-4xl font-black tracking-tighter text-white uppercase">
                        {name}
                    </h1>
                </div>
            </div>
            <div class="space-y-8 p-8 md:p-12">
                <div class="rounded-2xl border-l-4 border-tactical-yellow bg-tactical-yellow/10 p-4 shadow-lg backdrop-blur-md">
                    <p class="mb-1 font-mono text-xs font-bold tracking-widest text-tactical-yellow uppercase">
                        "Primary Threat"
                    </p>
                    <p class="text-body-md leading-relaxed text-on-surface-variant">
                        {threat_body}
                    </p>
                </div>
                <div>
                    {section_title("Telemetry")}
                    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
                        {vehicle_stat("Faction", faction)}
                        {vehicle_stat("Armor", armor)}
                        {vehicle_stat("Amphibious", amphib_label)}
                    </div>
                </div>
            </div>
        </div>
    }
}

/// The Edit and Delete buttons over the photograph's top corner.
#[cfg(target_arch = "wasm32")]
fn admin_toolbar(vehicle: &Vehicle, actions: DossierActions) -> impl IntoView + use<> {
    let edited = vehicle.clone();
    let deleted = vehicle.clone();
    view! {
        <div class="absolute top-4 right-4 flex gap-2">
            <button
                type="button"
                class=ACTION_BUTTON
                on:click=move |_| actions.on_edit.run(edited.clone())
            >
                <MaterialIcon name="edit" class="text-base" />
                "Edit"
            </button>
            <button
                type="button"
                class=DELETE_BUTTON
                on:click=move |_| actions.on_delete.run(deleted.clone())
            >
                <MaterialIcon name="delete" class="text-base" />
                "Delete"
            </button>
        </div>
    }
}

/// A small capitalised heading above a group of readouts.
#[cfg(target_arch = "wasm32")]
fn section_title(t: &'static str) -> impl IntoView {
    view! {
        <h2 class="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
            {t}
        </h2>
    }
}

/// One labelled readout in the telemetry grid.
#[cfg(target_arch = "wasm32")]
fn vehicle_stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <p class="font-mono text-[11px] tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 font-mono text-base text-white">{value}</p>
        </div>
    }
}

#[cfg(test)]
#[path = "tests/spec_drawer.rs"]
mod tests;
