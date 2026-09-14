//! The telemetry grid: population, theatre and environment, side by side.
//!
//! **Role:** renders the three columns beneath the panel header — how many players are on and
//! how the server is performing, the theatre tile and its current match, and the environment
//! readouts with the required mod configuration.
//! **Position:** the middle band of the frosted panel, between the header and the intelligence
//! strip.
//! **Signals & state:** reads the status accessor the panel built, so every value tracks the
//! live stream frame.
//! **Invariants:** every readout falls back to an em dash rather than to a zero when there is
//! no status, so an unreported server never looks like a reported one. The theatre label is
//! rendered only when the payload named a theatre.
#![allow(dead_code)]

use super::server_list::{v_bool, v_str};
use crate::v2::core::api::dto::ServerStatusDto;
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::datefmt::format_uptime;
use leptos::prelude::*;
use serde_json::Value;

/// Theatre tile image.
const THEATER_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuBJhklFaKKJXQ3-uOGwrugGr_URw1Dq_3Jslvkc3lEtT4ObLWKv52ipE-EQWEm3QF4HeoY5vA8NcYt_e87d76A14Z48tuHODNidNphecUVm_Zy7NLBRexvt9uUcFOBLTk3RbiSAetUEMYX2BmQMPU-BU-HvmweLf1P4-jc1CjC0jDdMMR-fzb5BVtNID-Ak1iW3MuGzWiO4LfZ4WIPy8Ijk3kcsqRFXVroQ_rZSJ8yw4se-gszeDoVOc8Vp9HL5qLcEAtnI4pFEC4I";

/// The frame rate at or above which this panel calls the server optimal.
///
/// A cosmetic label, deliberately not the backend's low-frame-rate alert floor: sharing one
/// number would either start calling a server just under the alert threshold optimal, or start
/// implying an alert fired when none did.
const FPS_OPTIMAL_FLOOR: f64 = 30.0;

/// The three-column telemetry grid for the chosen server.
///
/// `terrain_name` is the theatre the payload named, if any; `theater_alt` is the alternative
/// text for the tile image; `modpack` is the required mod configuration object; `live` reports
/// the current status.
pub(super) fn telemetry_grid(
    terrain_name: Option<String>,
    theater_alt: String,
    modpack: Option<Value>,
    live: impl Fn() -> Option<ServerStatusDto> + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 gap-8 border-b border-white/5 p-8 md:grid-cols-[1fr_2fr_1fr] md:divide-x md:divide-white/10">
            <div class="flex flex-col justify-center space-y-4 md:pr-8">
                <div>
                    <span class="mb-1 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                        "Active Personnel"
                    </span>
                    <div class="flex items-baseline gap-2">
                        <span class="font-mono text-[30px] font-bold leading-tight text-tertiary-container">
                            {move || live().map(|l| l.player_count).unwrap_or(0)}
                        </span>
                        <span class="font-mono text-[20px] font-semibold text-on-surface-variant">
                            "/ " {move || live().map(|l| l.max_players).unwrap_or(0)}
                        </span>
                    </div>
                </div>
                <div class="space-y-2 pt-2">
                    <div class="flex items-center justify-between text-code-md text-on-surface-variant">
                        <span>"Uptime:"</span>
                        <span>
                            {move || {
                                live()
                                    .map(|l| format_uptime(l.uptime_seconds))
                                    .unwrap_or_else(|| "—".into())
                            }}
                        </span>
                    </div>
                    <div class="flex items-center justify-between text-code-md text-on-surface-variant">
                        <span>"Server FPS:"</span>
                        <span class=move || {
                            if live().map(|l| l.server_fps >= FPS_OPTIMAL_FLOOR).unwrap_or(false)
                            {
                                "text-tactical-yellow"
                            } else {
                                "text-error"
                            }
                        }>
                            {move || {
                                live()
                                    .map(|l| {
                                        let opt = if l.server_fps >= FPS_OPTIMAL_FLOOR {
                                            "Optimal"
                                        } else {
                                            "Low"
                                        };
                                        // Default formatting prints a fractional rate with its
                                        // decimal and a whole one without; a fixed precision
                                        // would add a decimal place the original never showed.
                                        format!("{} ({opt})", l.server_fps)
                                    })
                                    .unwrap_or_else(|| "—".into())
                            }}
                        </span>
                    </div>
                </div>
            </div>

            <div class="flex flex-col justify-center md:px-8">
                <span class="mb-3 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                    "Theater of Operations"
                </span>
                <a href="/events" class="block focus:outline-none">
                    <div class="group relative aspect-[21/9] w-full cursor-pointer overflow-hidden rounded-lg border border-white/10 transition-all duration-300 hover:ring-2 hover:ring-primary hover:ring-offset-2 hover:ring-offset-background">
                        <img
                            alt=theater_alt.clone()
                            src=THEATER_IMAGE
                            class="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                        />
                        <div class="absolute inset-0 bg-gradient-to-t from-surface-container-highest/90 via-surface-container-highest/20 to-transparent"></div>
                        <div class="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                            <div>
                                {terrain_name.clone().map(|name| {
                                    view! {
                                        <span class="block text-label-md text-on-surface">
                                            {name}
                                        </span>
                                    }
                                })}
                                <span class=move || {
                                    if terrain_name.is_some() {
                                        "mt-0.5 block text-label-sm text-primary".to_string()
                                    } else {
                                        "block text-label-sm text-primary".to_string()
                                    }
                                }>
                                    {move || {
                                        live()
                                            .and_then(|l| l.current_match_id)
                                            .filter(|m| !m.is_empty())
                                            .map(|m| {
                                                let end = m.len().min(8);
                                                format!("Match {}", &m[..end])
                                            })
                                            .unwrap_or_else(|| "No Active Mission".into())
                                    }}
                                </span>
                            </div>
                            <MaterialIcon name="map" class="text-on-surface-variant" />
                        </div>
                    </div>
                </a>
            </div>

            <div class="flex flex-col justify-center space-y-6 md:pl-8">
                {env_row(
                    "schedule",
                    "text-primary",
                    "Simulated Time",
                    move || live().and_then(|l| l.ingame_time).unwrap_or_else(|| "—".into()),
                )}
                {env_row(
                    "rainy",
                    "text-tertiary-container",
                    "Conditions",
                    move || {
                        live().and_then(|l| l.ingame_weather).unwrap_or_else(|| "—".into())
                    },
                )}
                <div class="flex items-center gap-4">
                    <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                        <MaterialIcon name="verified" class="text-tactical-yellow" />
                    </div>
                    <div>
                        <span class="block text-label-sm uppercase text-on-surface-variant">
                            "Mod Configuration"
                        </span>
                        <span class="text-body-md text-on-surface">
                            {match modpack {
                                Some(mp) => {
                                    let label = format!(
                                        "{} v{}",
                                        v_str(&mp, "name"),
                                        v_str(&mp, "version"),
                                    );
                                    let synced = v_bool(&mp, "is_current");
                                    view! {
                                        {label}
                                        " "
                                        {synced
                                            .then(|| {
                                                view! {
                                                    <span class="text-[12px] text-on-surface-variant">
                                                        "(Synced)"
                                                    </span>
                                                }
                                            })}
                                    }
                                        .into_any()
                                }
                                None => view! { "No modpack required" }.into_any(),
                            }}
                        </span>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// One environment readout: a circled icon, its label, and a value that tracks the stream.
fn env_row(
    icon: &'static str,
    icon_class: &'static str,
    label: &'static str,
    value: impl Fn() -> String + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-4">
            <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                <MaterialIcon name=icon class=icon_class />
            </div>
            <div>
                <span class="block text-label-sm uppercase text-on-surface-variant">{label}</span>
                <span class="text-body-md text-on-surface">{move || value()}</span>
            </div>
        </div>
    }
}
