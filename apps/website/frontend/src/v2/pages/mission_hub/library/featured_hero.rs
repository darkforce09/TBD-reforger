//! The cinematic hero above the mission grid.
//!
//! **Role:** spotlights one operation — its art, briefing, badges and a button that opens its
//! dossier.
//! **Position:** the first block of the library body, above the search toolbar.
//! **Signals & state:** none of its own; it is handed the mission to spotlight and the callback
//! that opens the dossier sheet.
//! **Invariants:** the hero shows the newest mission across the whole library, which can be a
//! draft, so the "Live Operation" pulse is gated on the mission really being live; anything else
//! shows its own status in a muted chip rather than claiming to be the priority deployment.

use super::card_grid::{game_mode_label, mission_art_url, terrain_label};
use crate::v2::core::api::dto::MissionCard;
use crate::v2::core::ui::badge_class;
use leptos::prelude::*;

/// Fallback briefing for a spotlighted mission whose own briefing is empty.
pub(super) const FEATURED_BRIEFING_FALLBACK: &str = "Command has flagged this operation as the priority deployment. Review the dossier for objectives, ORBAT, and the armory loadout before committing forces to the field.";

/// The briefing to show for the spotlighted mission.
///
/// Whitespace-only is not authored content and takes the fallback; leading or trailing space
/// around real text is not emptiness and is kept.
pub(super) fn featured_briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => FEATURED_BRIEFING_FALLBACK.into(),
    }
}

/// The hero, or nothing when the library has no mission to spotlight.
pub(super) fn featured_hero(
    featured: Option<MissionCard>,
    open_preview: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    featured.map(|f| {
                    let art = mission_art_url(f.thumbnail_url.as_deref());
                    let brief = featured_briefing_text(f.briefing.as_deref());
                    let fid = f.id.clone();
                    let is_live = f.status == "live";
                    let status_label = crate::v2::pages::mission_hub::overview::mission_status_label(&f.status);
                    view! {
                        <section class="relative mb-8 flex min-h-[320px] flex-col overflow-hidden rounded-2xl border border-white/10 bg-black/30 lg:flex-row">
                            <div class="relative z-10 flex w-full flex-col justify-center gap-4 p-8 lg:w-3/5">
                                {if is_live {
                                    view! {
                                        <div class="flex items-center gap-2 font-mono text-label-sm tracking-widest text-error-alert uppercase">
                                            <span class="relative flex h-2.5 w-2.5">
                                                <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-error-alert opacity-60"></span>
                                                <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-error-alert"></span>
                                            </span>
                                            "Live Operation"
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="flex items-center gap-2 font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                                            {status_label}
                                        </div>
                                    }
                                        .into_any()
                                }}
                                <h2 class="text-4xl font-black tracking-tighter text-on-surface uppercase xl:text-5xl">
                                    {f.title.clone()}
                                </h2>
                                <p class="max-w-prose text-body-md text-on-surface-variant line-clamp-3">
                                    {brief}
                                </p>
                                <div class="flex flex-wrap items-center gap-2">
                                    <span class=badge_class(
                                        "primary",
                                    )>{game_mode_label(&f.game_mode).to_string()}</span>
                                    <span class=badge_class("neutral")>{terrain_label(&f.terrain)}</span>
                                    <span class=badge_class(
                                        "tertiary",
                                    )>{f.max_players} " OPERATORS"</span>
                                </div>
                                <div>
                                    <button
                                        type="button"
                                        on:click=move |_| open_preview(fid.clone())
                                        class="mt-2 rounded-lg bg-primary px-6 py-3 font-mono text-label-md font-semibold tracking-wider text-on-primary uppercase transition-transform hover:scale-[1.02]"
                                    >
                                        "[ View Dossier ]"
                                    </button>
                                </div>
                            </div>
                            <div class="absolute inset-0 lg:relative lg:inset-auto lg:w-2/5">
                                <img
                                    src=art
                                    alt=""
                                    class="h-full w-full object-cover opacity-60 mix-blend-luminosity"
                                />
                                <div class="absolute inset-0 bg-gradient-to-r from-surface to-transparent"></div>
                            </div>
                        </section>
        }
    })
}
