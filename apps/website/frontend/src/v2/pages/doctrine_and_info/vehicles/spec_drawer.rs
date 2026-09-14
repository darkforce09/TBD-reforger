//! The detail half of the vehicle index: one vehicle's identification dossier.
//!
//! **Role:** renders the selected vehicle — its photograph or a placeholder, the armour,
//! amphibious and faction chips, its primary threat note, and a small telemetry grid.
//! **Position:** the detail pane of the vehicle index's split view.
//! **Signals & state:** none — it is handed an owned row and renders it.
//! **Invariants:** every field is optional on the wire; an absent image falls back to an icon,
//! an absent amphibious value drops the chip, and an absent threat shows a fixed line instead.

use super::helpers::vstr;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// The neutral chip, used for the armour class.
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
/// The warning chip, used for an amphibious vehicle.
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";
/// The success chip, used for a vehicle that is not amphibious.
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
/// The primary chip, used for the faction.
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";

/// The chip class for an amphibious value.
///
/// A recognised yes reads as a warning, a recognised no as a success, and anything else —
/// including an unrecognised spelling — as neutral.
fn amphib_badge(amphibious: &str) -> &'static str {
    match amphibious.trim().to_ascii_lowercase().as_str() {
        "yes" | "y" | "true" => BADGE_WARNING,
        "no" | "n" | "false" => BADGE_SUCCESS,
        _ => BADGE_NEUTRAL,
    }
}

/// The identification dossier for one vehicle.
pub(super) fn dossier(v: Value) -> impl IntoView {
    let name = vstr(&v, "name");
    let faction = vstr(&v, "faction");
    let armor = vstr(&v, "armor_type");
    let amphib = vstr(&v, "amphibious");
    let threat = vstr(&v, "primary_threat");
    let image = vstr(&v, "profile_image_url");

    let hero = if image.is_empty() {
        view! {
            <div class="flex h-full w-full items-center justify-center bg-surface-container-low">
                <MaterialIcon name="directions_car" class="text-7xl text-outline" />
            </div>
        }
        .into_any()
    } else {
        view! { <img src=image alt="" class="h-full w-full object-cover" /> }.into_any()
    };

    let amphib_label = if amphib.is_empty() {
        "—".to_string()
    } else {
        amphib.clone()
    };
    let threat_body = if threat.is_empty() {
        "No primary threat recorded.".to_string()
    } else {
        threat.clone()
    };

    view! {
        <div>
            <div class="relative h-72 w-full overflow-hidden">
                {hero}
                <div class="absolute inset-0 bg-gradient-to-t from-surface-dim to-transparent"></div>
                <div class="absolute right-8 bottom-6 left-8">
                    <div class="mb-3 flex flex-wrap items-center gap-2">
                        <span class=BADGE_NEUTRAL>"ARMOR: "{armor.clone()}</span>
                        {if !amphib.is_empty() {
                            view! {
                                <span class=amphib_badge(&amphib)>"AMPHIB: "{amphib.clone()}</span>
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

/// A small capitalised heading above a group of readouts.
fn section_title(t: &'static str) -> impl IntoView {
    view! {
        <h2 class="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
            {t}
        </h2>
    }
}

/// One labelled readout in the telemetry grid.
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
