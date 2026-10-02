//! The full-width banner at the top of the dashboard.
//!
//! **Role:** renders the countdown to the next operation over a cinematic backdrop, with the
//! operation's name and theatre beneath it and a link into its hub.
//! **Position:** the first child of the dashboard column, above the card grid.
//! **Signals & state:** none — the next-operation payload arrives owned and is read once.
//! **Invariants:** with no upcoming operation the banner still renders, with the headline and
//! the strapline replaced by their empty-state text and no link. The countdown reports
//! `LIVE NOW` on its own once the start time has passed, so that label is not prefixed.
#![allow(dead_code)]

use super::helpers::vstr;
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::countdown::countdown_label;
use leptos::prelude::*;
use serde_json::Value;

/// Backdrop image for the banner.
const HERO_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuB_SlrhFHaG9jlm7NfoEUTrANNfG_-m0cqYcJVwKZ1pAUA_LTEnwP1zyNasVKfTgKdnX14ssTtYpEc3I1qn0UaEjwwEQyuAGxherp9Eu5rIpF4afr0sjFAUSjc9Z5NpB2xub7NkJCKNYCkkFsIa25L2e5QrbN4lEOZHeGZeLxpbVtQC8WATlT2skffHxtraZAi95LpXOqnuyLkxHIoJOHtxsFj2rJ4xCywZTnNZy_bJSzmLgPaun0eZsYw-Prx2nJ2GeJMP72x2l-4";

/// The hero banner for `next`, the upcoming operation the payload named.
///
/// Reads `start_time`, `name`, `terrain` and `event_id` from the operation object. Returns the
/// banner element; there are no side effects.
pub(super) fn hero_banner(next: Option<Value>) -> impl IntoView {
    view! {
        <div class="glass border-glow relative flex min-h-[300px] flex-col justify-end overflow-hidden rounded-xl p-8">
            <div class="absolute inset-0 z-0">
                <img
                    alt="Operation theater"
                    src=HERO_IMAGE
                    class="h-full w-full object-cover opacity-40 mix-blend-overlay"
                />
                <div class="absolute inset-0 bg-gradient-to-t from-surface-container-lowest via-surface-container-lowest/80 to-transparent"></div>
            </div>
            <div class="relative z-10 flex w-full flex-wrap items-end justify-between gap-4">
                <div class="flex flex-col">
                    <h2 class="text-glow mb-2 font-mono text-5xl font-bold tracking-tighter text-primary md:text-7xl">
                        {match &next {
                            Some(n) => {
                                let label = countdown_label(&vstr(n, "start_time"));
                                if label == "LIVE NOW" {
                                    label
                                } else {
                                    format!("T-MINUS {label}")
                                }
                            }
                            None => "NO UPCOMING OPS".to_string(),
                        }}
                    </h2>
                    <p class="flex items-center gap-2 text-sm tracking-widest text-on-surface uppercase opacity-80">
                        <span class="h-2 w-2 animate-pulse rounded-full bg-primary"></span>
                        {match &next {
                            Some(n) => {
                                format!("OPERATION: {} — {}", vstr(n, "name"), vstr(n, "terrain"))
                            }
                            None => "Check the event schedule for new operations.".to_string(),
                        }}
                    </p>
                </div>
                {next
                    .as_ref()
                    .map(|n| {
                        let href = format!("/events/{}", vstr(n, "event_id"));
                        view! {
                            <a
                                href=href
                                class="group flex items-center gap-2 rounded-lg border border-primary/50 bg-surface/50 px-6 py-3 text-sm font-bold tracking-widest text-primary uppercase backdrop-blur-md transition-all hover:bg-primary/20 active:scale-95"
                            >
                                "Open Operation Hub"
                                <MaterialIcon
                                    name="arrow_forward"
                                    class="transition-transform group-hover:translate-x-1"
                                />
                            </a>
                        }
                    })}
            </div>
        </div>
    }
}
