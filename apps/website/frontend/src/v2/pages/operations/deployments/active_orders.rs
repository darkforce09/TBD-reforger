//! The active-orders banner: the caller's next deployment, and the queue behind it.
//!
//! **Role:** renders the soonest upcoming deployment as the banner — name, reservation and attendance,
//! local start time, countdown, terrain, assigned slot and the two links into the operation —
//! with the remaining deployments listed beneath it. Reads typed upcoming deployment rows.
//! **Position:** inside the banner section at the top of the right-hand column.
//! **Signals & state:** none — the banner is built from the values handed to it.
//! **Invariants:** a link is rendered only when the ids it needs are actually on the wire; a row
//! without an operation to link to is inert markup rather than an anchor to a partial path.
#![allow(dead_code)]

use crate::v2::core::api::dto::DeploymentUpcoming;
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::countdown::countdown_label;
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

/// Reservation and attendance badges use the same status colors.
fn state_variant(state: &str) -> &'static str {
    match state {
        "registered" | "attended" => "success",
        "waitlisted" => "warning",
        "withdrawn" | "no_show" => "error",
        _ => "neutral",
    }
}

/// A terrain identifier with its first letter capitalised, or an em dash when it is empty.
fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "—".into(),
    }
}

/// "BLUFOR · Command · Platoon Leader" from whichever of `faction` / `squad` / `role` the backend
/// actually filled — an unassigned registration carries none of the three, and joining blindly would
/// render a row of bare separators.
fn slot_line(upcoming: &DeploymentUpcoming) -> String {
    [&upcoming.faction, &upcoming.squad, &upcoming.role]
        .into_iter()
        .map(String::as_str)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The active-orders banner.
///
/// `upcoming` is ordered soonest-first by the backend, so the first row is the banner and the
/// rest queue beneath it without a second heading — the banner already carries one.
pub(super) fn active_orders(upcoming: Vec<DeploymentUpcoming>) -> impl IntoView {
    let mut it = upcoming.into_iter();
    let Some(next) = it.next() else {
        // `has_active` is checked by the caller; this arm exists so the function is total.
        return ().into_any();
    };
    let rest: Vec<DeploymentUpcoming> = it.collect();
    let slot = slot_line(&next);
    let name = if next.name.is_empty() {
        "Untitled Operation".to_string()
    } else {
        next.name
    };
    let when = format_local_datetime(&next.start_time);
    let countdown = countdown_label(&next.start_time);
    let terrain = terrain_label(&next.terrain);
    let state = next.reservation_state;
    let attendance = next.attendance_state;
    let event_id = next.event_id;
    let emid = next.event_mission_id;
    // The slotting link is rendered only when both ids are on the wire: a registration with no
    // mission id has no order of battle to change.
    let orbat_href = (!event_id.is_empty() && !emid.is_empty())
        .then(|| format!("/events/{event_id}/missions/{emid}/orbat"));
    let hub_href = (!event_id.is_empty()).then(|| format!("/events/{event_id}"));
    view! {
        <div class="flex flex-col gap-5">
            <div class="flex flex-col gap-3">
                <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
                    <h3 class="text-3xl font-black uppercase tracking-tight text-on-surface">
                        {name}
                    </h3>
                    {(!state.is_empty())
                        .then(|| {
                            view! { <span class=badge_class(state_variant(&state))>"Reservation: "{state.clone()}</span> }
                        })}
                    {attendance.map(|attendance| {
                        let class = badge_class(state_variant(&attendance));
                        view! { <span class=class>"Attendance: "{attendance}</span> }
                    })}
                </div>
                <div class="flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                    <span>{when}</span>
                    <span class="text-primary">"T-MINUS "{countdown}</span>
                    <span>{terrain}</span>
                </div>
                {(!slot.is_empty())
                    .then(|| {
                        view! {
                            <span class="w-fit rounded-md border border-primary/30 bg-primary/10 px-2.5 py-1 font-mono text-xs tracking-widest text-primary uppercase">
                                "Assigned slot: "
                                {slot.clone()}
                            </span>
                        }
                    })}
            </div>
            <div class="flex flex-wrap gap-3">
                {orbat_href
                    .map(|href| {
                        view! {
                            <a
                                href=href
                                class="inline-flex items-center gap-2 rounded-full border border-primary/50 bg-surface/50 px-5 py-2.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/20"
                            >
                                <MaterialIcon name="tune" class="text-base" />
                                "Modify Assignment"
                            </a>
                        }
                    })}
                {hub_href
                    .map(|href| {
                        view! {
                            <a
                                href=href
                                class="inline-flex items-center gap-2 rounded-full border border-white/10 px-5 py-2.5 font-mono text-xs tracking-widest text-on-surface uppercase transition hover:bg-white/5"
                            >
                                <MaterialIcon name="open_in_new" class="text-base" />
                                "Operation Hub"
                            </a>
                        }
                    })}
            </div>
            {(!rest.is_empty())
                .then(|| {
                    view! {
                        <div class="flex flex-col gap-2 border-t border-white/10 pt-4">
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Also Awaiting Deployment"
                            </span>
                            {rest
                                .into_iter()
                                .map(|u| {
                                    let n = if u.name.is_empty() {
                                        "Untitled Operation".to_string()
                                    } else {
                                        u.name
                                    };
                                    let s = u.start_time;
                                    let st = u.reservation_state;
                                    let attendance = u.attendance_state;
                                    let eid = u.event_id;
                                    let row = view! {
                                        <>
                                            <span class="min-w-0 flex-1 truncate text-on-surface">{n}</span>
                                            <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                {format_local_datetime(&s)}
                                            </span>
                                            {(!st.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <span class=badge_class(state_variant(&st))>"Reservation: "{st.clone()}</span>
                                                    }
                                                })}
                                            {attendance.map(|attendance| {
                                                let class = badge_class(state_variant(&attendance));
                                                view! { <span class=class>"Attendance: "{attendance}</span> }
                                            })}
                                        </>
                                    };
                                    // A row is a link when it has an event to link to, and inert
                                    // markup when it does not — never an anchor to "/events/".
                                    if eid.is_empty() {
                                        view! {
                                            <div class="flex items-center gap-3 rounded-lg border border-white/10 px-4 py-2.5 text-sm">
                                                {row}
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <a
                                                href=format!("/events/{eid}")
                                                class="flex items-center gap-3 rounded-lg border border-white/10 px-4 py-2.5 text-sm transition hover:bg-white/[0.03]"
                                            >
                                                {row}
                                            </a>
                                        }
                                            .into_any()
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })}
        </div>
    }
    .into_any()
}
