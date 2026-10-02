//! The operation's places as the viewer sees them: each pool, the operation-wide remainder, and
//! the viewer's own access.
//!
//! **Role:** renders the Places panel of the hub body — one row per pool (member, guest, open)
//! with what it can give, why it gives nothing when closed, and when it opens or opened; the
//! places left under the operation-wide limit; the pool the viewer's place comes from first; and
//! the notices for a partial view and a pending membership verification.
//! **Position:** below the hero of the hub body, above the mission dossiers, on both the operation
//! route and the schedule's detail column.
//! **Signals & state:** none; built once from the dossier.
//! **Invariants:** every line comes out of the dossier; a pool the dossier does not carry is not
//! shown. An opening time is shown in the viewer's own zone with its UTC time beside it. The line
//! wording is pure and takes the local rendering as a parameter, so it is tested without a browser.

use super::refusal_notices::pool_name;
use crate::v2::core::api::dto::{EventHub, ReservationQuotaAvailability};
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::format_local_datetime;
use crate::v2::core::utils::utc_timestamp::utc_label;
use leptos::prelude::*;

/// What one pool can give right now, as a line: `Open now · 1 of 2 places left`,
/// `Full · 2 of 2 taken`, `Not open yet · 5 of 5 places left`.
pub(crate) fn pool_availability_line(pool: &ReservationQuotaAvailability) -> String {
    let capacity = match (pool.seat_limit, pool.remaining) {
        (Some(limit), Some(left)) => format!("{left} of {limit} places left"),
        (Some(limit), None) => format!("limit {limit}"),
        (None, _) => format!("no pool limit, {} taken", pool.allocated),
    };
    match pool.closed_reason.as_deref() {
        None if pool.open => format!("Open now · {capacity}"),
        Some("not_yet_open") => format!("Not open yet · {capacity}"),
        Some("no_places") => "Closed · this pool has no places".to_string(),
        Some("full") => match pool.seat_limit {
            Some(limit) => format!("Full · {} of {limit} taken", pool.allocated),
            None => "Full".to_string(),
        },
        _ => "Closed".to_string(),
    }
}

/// When a pool opens or opened, in the viewer's zone with the UTC time beside it; `local_time`
/// renders an instant in that zone. A pool without places has no opening worth stating.
pub(crate) fn pool_opening_line(
    pool: &ReservationQuotaAvailability,
    local_time: impl Fn(&str) -> String,
) -> Option<String> {
    let at = format!(
        "{} ({})",
        local_time(&pool.opens_at),
        utc_label(&pool.opens_at)
    );
    match pool.closed_reason.as_deref() {
        Some("no_places") => None,
        Some("not_yet_open") => Some(format!("Opens {at}")),
        None | Some("full") => Some(format!("Opened {at}")),
        Some(_) => Some(format!("Opening time {at}")),
    }
}

/// The badge variant a pool's state is shown with.
pub(crate) fn pool_badge_variant(pool: &ReservationQuotaAvailability) -> &'static str {
    match pool.closed_reason.as_deref() {
        None if pool.open => "success",
        Some("not_yet_open") => "warning",
        _ => "neutral",
    }
}

/// The operation-wide remainder, as a line.
pub(crate) fn remaining_places_line(remaining: Option<i64>) -> String {
    match remaining {
        None => "No operation-wide limit".to_string(),
        Some(left) if left <= 0 => "The operation is full".to_string(),
        Some(1) => "1 place left in the operation".to_string(),
        Some(left) => format!("{left} places left in the operation"),
    }
}

/// Which pool the viewer's place comes from, as a sentence.
pub(crate) fn quota_class_line(quota_class: &str) -> String {
    let own = pool_name(Some(quota_class)).to_lowercase();
    format!("Your place comes from {own} first, then from open places once they open.")
}

/// The Places panel of the hub body.
pub(crate) fn places_panel(hub: &EventHub) -> impl IntoView {
    let own_pool = hub.viewer_access.quota_class.clone();
    let pools: Vec<(String, String, Option<String>, String, bool)> = hub
        .reservation_quotas
        .iter()
        .map(|pool| {
            (
                pool_name(Some(&pool.quota_kind)),
                pool_availability_line(pool),
                pool_opening_line(pool, format_local_datetime),
                badge_class(pool_badge_variant(pool)),
                pool.quota_kind == own_pool,
            )
        })
        .collect();
    let remaining = remaining_places_line(hub.remaining_event_places);
    let class_line = quota_class_line(&own_pool);
    let partial = hub.viewer_access.visibility != "full";
    let pending = hub.viewer_access.membership_verification_pending;
    view! {
        <section
            class="mb-8 rounded-xl border border-outline-variant/30 bg-surface-container p-6"
            data-testid="event-places"
        >
            <h2 class="mb-3 text-label-md text-on-surface-variant uppercase tracking-wide">
                "Places"
            </h2>
            <ul class="space-y-2">
                {pools
                    .into_iter()
                    .map(|(name, line, opening, badge, own)| {
                        view! {
                            <li class="text-sm">
                                <div class="flex flex-wrap items-center justify-between gap-2">
                                    <span class="flex items-center gap-2 text-on-surface">
                                        {name}
                                        {own.then(|| view! { <span class=badge_class("primary")>"Your pool"</span> })}
                                    </span>
                                    <span class=badge>{line}</span>
                                </div>
                                {opening.map(|opening| view! { <p class="mt-0.5 text-xs text-on-surface-variant">{opening}</p> })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <p class="mt-3 font-mono text-sm text-on-surface">{remaining}</p>
            <p class="mt-1 text-sm text-on-surface-variant">{class_line}</p>
            {partial
                .then(|| {
                    view! {
                        <p class="mt-3 flex items-start gap-2 text-sm text-on-surface-variant">
                            <MaterialIcon name="visibility" class="text-base" />
                            "You see only the missions and seats open to you, and not the operation briefing."
                        </p>
                    }
                })}
            {pending
                .then(|| {
                    view! {
                        <p class="mt-3 flex items-start gap-2 text-sm text-tactical-yellow">
                            <MaterialIcon name="pending" class="text-base" />
                            "Your Discord membership is being verified; seats that rely on it open to you once verification completes."
                        </p>
                    }
                })}
        </section>
    }
}
