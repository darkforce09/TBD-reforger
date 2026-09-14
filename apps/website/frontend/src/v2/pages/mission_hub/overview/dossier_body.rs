//! The shared, read-only mission dossier: badges, briefing, detail grid and the faction armory.
//!
//! **Role:** the one rendering of a mission's facts, and the small formatters the rest of the hub
//! reads it through — the game mode, the terrain, the status label and the detail rows.
//! **Position:** the body of the `/missions/:id` route, and the same block inside the library's
//! slide-over dossier.
//! **Signals & state:** one local signal for the selected armory faction, defaulting to the first
//! faction the armory names.
//! **Invariants:** read-only, deliberately — it renders inside a slide-over, so an editor here
//! would be a form in an overlay that another overlay then stacks on. The armory tabs are built
//! from the armory's own rows, so they show whatever key is stored, joining or not. The status
//! cell goes through [`mission_status_label`], the one status formatter on the platform, so a
//! returned mission never reads as a raw database token.

use super::intel_briefing::briefing_section;
use crate::v2::core::api::dto::MissionDetail;
use leptos::prelude::*;
use serde_json::Value;

// The base text size is dropped by the utility merge against the trailing colour class, so it is
// not restated in any of the three.
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
const BADGE_TERTIARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tertiary/30 bg-tertiary/10 text-tertiary";

/// The display name of a game mode; anything unrecognised is passed through unchanged.
pub(super) fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// A terrain name with its first character capitalised; an em dash when there is none.
pub(super) fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The one mission-status label on the platform.
///
/// The status column is an enum of five values, and this names every one of them. The card badge
/// takes its label from here too, so there is exactly one mapping to change; the badge keeps only
/// the colour variant, which is a badge concern the detail grid has no use for.
///
/// The fallback arm is unreachable defence over those five values, and deliberately not a
/// passthrough: an unknown status is uppercased and de-underscored, so the worst case reads as a
/// name rather than leaking a raw database token.
pub(crate) fn mission_status_label(status: &str) -> String {
    match status {
        "draft" => "Draft".to_string(),
        "pending_approval" => "Open for review".to_string(),
        "live" => "Live".to_string(),
        "rejected" => "Returned".to_string(),
        "archived" => "Archived".to_string(),
        other => other.replace('_', " ").to_uppercase(),
    }
}

/// The dossier detail grid, as `(label, value)` rows.
///
/// The grid is built from this, so the status cell has a surface a test can drive: a view macro
/// cannot be asserted on from a native test, and as long as the mapping lived inline in the markup
/// the only instrument available was a source scan.
pub(super) fn detail_rows(m: &MissionDetail) -> Vec<(&'static str, String)> {
    vec![
        ("Weather", m.weather.clone()),
        ("Time", m.time_of_day.clone()),
        ("Max Players", m.max_players.to_string()),
        ("Status", mission_status_label(&m.status)),
    ]
}

/// The briefing to show for a mission.
///
/// Whitespace-only is not authored content and takes the empty affordance; leading or trailing
/// space around real text is not emptiness and is kept.
pub(super) fn tactical_briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => "No briefing provided.".into(),
    }
}

/// The shared dossier content, rendered here and inside the library's slide-over.
pub fn dossier_body(m: &MissionDetail) -> impl IntoView {
    let v_badge = m
        .current_version
        .as_ref()
        .map(|v| view! { <span class=BADGE_TERTIARY>"v"{v.semver.clone()}</span> });
    // The armory faction tabs, one per distinct faction the stored rows name.
    let armory = m.armory.clone();
    let factions: Vec<String> = {
        let mut seen = Vec::new();
        for a in &armory {
            if let Some(f) = a.get("faction").and_then(|v| v.as_str()) {
                if !seen.iter().any(|s: &String| s == f) {
                    seen.push(f.to_string());
                }
            }
        }
        seen
    };
    let faction_sel = RwSignal::new(None::<String>);
    // A stored value keeps the resolver closure `Copy`, so both the tabs and the rows can use it.
    let default_faction = StoredValue::new(factions.first().cloned());
    let factions_for_tabs = factions.clone();
    let active_faction = move || faction_sel.get().or_else(|| default_faction.get_value());
    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap gap-2">
                <span class=BADGE_PRIMARY>{game_mode_label(&m.game_mode).to_string()}</span>
                <span class=BADGE_NEUTRAL>{terrain_label(&m.terrain)}</span>
                {v_badge}
            </div>
            {briefing_section(m)}

            <dl class="grid grid-cols-1 gap-8 md:grid-cols-2">
                {detail_rows(m).into_iter().map(|(l, v)| detail(l, v)).collect_view()}
            </dl>

            {(!factions.is_empty())
                .then(move || {
                    let af_tabs = active_faction;
                    let af_rows = active_faction;
                    let armory_rows = armory.clone();
                    view! {
                        <section>
                            <h3 class="mb-2 text-label-md text-on-surface-variant uppercase">
                                "The Armory"
                            </h3>
                            <div class="mb-3 flex gap-2">
                                {factions_for_tabs
                                    .iter()
                                    .map(|f| {
                                        let f_click = f.clone();
                                        let f_active = f.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| faction_sel.set(Some(f_click.clone()))
                                                class=move || {
                                                    crate::v2::core::ui::cn(
                                                        &[
                                                            "rounded-lg px-3 py-1.5 text-label-md",
                                                            if af_tabs().as_deref() == Some(f_active.as_str()) {
                                                                "bg-primary text-on-primary"
                                                            } else {
                                                                "bg-surface-container text-on-surface-variant"
                                                            },
                                                        ],
                                                    )
                                                }
                                            >
                                                {f.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="grid gap-2">
                                {move || {
                                    let af = af_rows();
                                    armory_rows
                                        .iter()
                                        .filter(|a| {
                                            a.get("faction").and_then(|v| v.as_str())
                                                == af.as_deref()
                                        })
                                        .map(|item| {
                                            let name = item
                                                .get("item_name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string();
                                            let qty = match item.get("quantity") {
                                                Some(Value::Number(n)) => format!("x{n}"),
                                                _ => "∞".to_string(),
                                            };
                                            view! {
                                                <div class="flex justify-between rounded-lg border border-outline-variant/30 bg-surface-container p-3 text-label-md">
                                                    <span class="text-on-surface">{name}</span>
                                                    <span class="text-on-surface-variant">{qty}</span>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </section>
                    }
                })}
        </div>
    }
}

/// One cell of the detail grid.
fn detail(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <dt class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </dt>
            <dd class="mt-1 text-headline-sm text-on-surface">{value}</dd>
        </div>
    }
}
