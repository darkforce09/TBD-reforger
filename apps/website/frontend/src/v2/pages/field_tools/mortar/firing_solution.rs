//! The firing-solution card — the numbers the gun line is laid with.
//!
//! **Role:** renders the computed distance, azimuth, elevation, propellant charge and time of
//! flight, and says whether those numbers survive a reload.
//! **Position:** floats over the bottom-right corner of the map panel on `/tools/mortar`.
//! **Signals & state:** reads the page's `solution` and `weapon` signals; writes nothing.
//! **Invariants:** a missing charge or time of flight renders as an em dash and never as a
//! fabricated zero — a solution stored before those columns existed genuinely has neither, and
//! `0.0 s` is indistinguishable from a real flight time.

use super::grid::locale_int;
use super::saved_fires::Shown;
use leptos::prelude::*;

/// The card's own class. The base is a positioned glass panel; the border accent marks it as the
/// answer rather than as an input.
const CARD_SOLUTION: &str = "flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass absolute right-4 bottom-4 w-72 border-t-2 border-tertiary";

/// The solution card. Falls back to the picked weapon's name in the heading until a solution
/// exists, so the card names the tube it is about to answer for.
pub(super) fn firing_solution(
    solution: RwSignal<Option<Shown>>,
    weapon: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class=CARD_SOLUTION>
            <h2 class="text-sm font-semibold text-primary">
                "Firing Solution — "
                {move || {
                    solution.get().map(|s| s.weapon_system).unwrap_or_else(|| weapon.get())
                }}
            </h2>
            {move || match solution.get() {
                Some(s) => {
                    view! {
                        <p class=move || {
                            if s.saved_at.is_some() {
                                "text-xs text-success"
                            } else {
                                "text-xs text-tactical-yellow"
                            }
                        }>
                            {if s.saved_at.is_some() {
                                "Saved — survives a reload"
                            } else {
                                "Not saved — lost on reload"
                            }}
                        </p>
                        <dl class="mt-3 space-y-2 font-mono text-sm">
                            <div class="flex justify-between">
                                <dt class="text-on-surface-variant">"Distance"</dt>
                                <dd>{locale_int(s.distance_m as f64)} " m"</dd>
                            </div>
                            <div class="flex justify-between">
                                <dt class="text-on-surface-variant">"Azimuth"</dt>
                                <dd>{format!("{:.1}°", s.azimuth_deg)}</dd>
                            </div>
                            <div class="flex justify-between">
                                <dt class="text-on-surface-variant">"Elevation"</dt>
                                <dd class="text-primary">{s.elevation_mils} " mils"</dd>
                            </div>
                            // Charge and time of flight come off a stored row as well
                            // as off a live solve, so both read the same after a reload
                            // as they did when they were computed. The em dash is
                            // reserved for a row saved before those columns existed,
                            // which genuinely has neither.
                            <div class="flex justify-between">
                                <dt class="text-on-surface-variant">"Charge"</dt>
                                <dd
                                    title=move || {
                                        if s.charge.is_some() {
                                            ""
                                        } else {
                                            "saved before charge was stored — recalculate for it"
                                        }
                                    }
                                >
                                    {match s.charge {
                                        Some(c) => c.to_string(),
                                        None => "—".to_string(),
                                    }}
                                </dd>
                            </div>
                            <div class="flex justify-between">
                                <dt class="text-on-surface-variant">"TOF"</dt>
                                <dd
                                    title=move || {
                                        if s.time_of_flight_s.is_some() {
                                            ""
                                        } else {
                                            "saved before time of flight was stored — recalculate for it"
                                        }
                                    }
                                >
                                    {match s.time_of_flight_s {
                                        Some(t) => format!("{t:.1} s"),
                                        None => "—".to_string(),
                                    }}
                                </dd>
                            </div>
                        </dl>
                    }
                        .into_any()
                }
                None => {
                    view! {
                        <p class="mt-3 text-xs text-on-surface-variant">
                            "Enter coordinates and calculate to see solution."
                        </p>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
