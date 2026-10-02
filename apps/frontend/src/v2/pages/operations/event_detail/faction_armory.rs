//! One faction's card inside a mission dossier: its uniforms and its armory.
//!
//! **Role:** renders the per-faction card — the uniform silhouettes and, when the mission's
//! armory carries items for that faction, the item list with its quantities — and owns the
//! ordering every faction list on this page is sorted by.
//! **Position:** the "Faction Dossiers" grid inside a mission dossier card, and the faction tab
//! order in the slotting selector.
//! **Signals & state:** none — the card is built from the values handed to it.
//! **Invariants:** the card shows only what the dossier serves. A faction with no armory items
//! gets uniforms and nothing else, never a stand-in inventory.
#![allow(dead_code)]

use leptos::prelude::*;

/// A uniform silhouette, so the card always has a frame to draw even with no artwork to show.
///
/// This is art standing in for missing art, not a value standing in for a missing fact: a grey
/// silhouette asserts nothing about a faction, where a made-up vehicle roster would.
const PLACEHOLDER_UNIFORM: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='80' height='120'><rect width='80' height='120' fill='%23242a3a'/><circle cx='40' cy='38' r='15' fill='%233a4252'/><rect x='18' y='56' width='44' height='56' rx='9' fill='%233a4252'/></svg>";

/// Rank a faction name into its side: western first, eastern second, independent third, and
/// everything unrecognised last.
///
/// The name is lowercased and tokenized on non-alphanumerics; single-word markers match whole
/// tokens, multi-word ones match as substrings.
fn faction_side(name: &str) -> u8 {
    let lower = name.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has = |t: &str| tokens.contains(&t);
    if [
        "blufor", "bluefor", "nato", "usmc", "usa", "west", "western",
    ]
    .iter()
    .any(|t| has(t))
        || lower.contains("us army")
    {
        0
    } else if [
        "opfor", "ussr", "soviet", "russia", "csat", "east", "eastern",
    ]
    .iter()
    .any(|t| has(t))
    {
        1
    } else if [
        "indfor",
        "independent",
        "guer",
        "guerrilla",
        "resistance",
        "civ",
        "civilian",
    ]
    .iter()
    .any(|t| has(t))
    {
        2
    } else {
        99
    }
}

/// The faction render order used everywhere on this page: by side, then alphabetically inside a
/// side, so a mission reads the same way in its dossier and in its slotting tabs.
pub(super) fn sort_factions(mut factions: Vec<String>) -> Vec<String> {
    factions.sort_by(|a, b| faction_side(a).cmp(&faction_side(b)).then_with(|| a.cmp(b)));
    factions
}

/// One faction's dossier card: its name, the uniform silhouettes, and — only when the mission
/// serves items for it — the armory list with each item's quantity or an infinity marker.
pub(super) fn faction_dossier_card(
    faction: String,
    items: Vec<crate::v2::core::api::dto::ArmoryItem>,
) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border-subtle bg-surface-container p-3">
            <h5 class="mb-3 text-sm font-semibold">{faction}</h5>

            <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                "Uniforms"
            </span>
            <div class="mb-3 flex gap-2">
                {(0..3)
                    .map(|_| {
                        view! {
                            <img
                                src=PLACEHOLDER_UNIFORM
                                alt=""
                                class="aspect-[2/3] w-12 rounded-md border border-white/10 object-cover"
                            />
                        }
                    })
                    .collect_view()}
            </div>

            {(!items.is_empty())
                .then(|| {
                    view! {
                        <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                            "Armory"
                        </span>
                        <ul class="space-y-1 text-sm">
                            {items
                                .into_iter()
                                .map(|it| {
                                    let qty = it
                                        .quantity
                                        .map(|q| format!("x{q}"))
                                        .unwrap_or_else(|| "∞".to_string());
                                    view! {
                                        <li class="flex justify-between text-on-surface-variant">
                                            <span>{it.item_name}</span>
                                            <span>{qty}</span>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                })}
        </div>
    }
}
