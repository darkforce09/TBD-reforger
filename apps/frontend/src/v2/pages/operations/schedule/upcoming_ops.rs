//! One operation, as a card in the master list.
//!
//! **Role:** renders a single upcoming operation as a selectable card — its local start time,
//! lifecycle badge, mission and slot counts, countdown or lock marker, and fill bar — and holds
//! the readers that pull those fields out of an untyped event row.
//! **Position:** repeated down the master column of the schedule's split pane, under the
//! "Upcoming Ops" header.
//! **Signals & state:** writes the caller's `picked` signal on click and reads its `selected_id`
//! memo to decide whether the card wears the selected treatment.
//! **Invariants:** the fill bar is driven by the server's own `percent`, clamped to 0..100 —
//! `total_slots` is zero until missions are attached, so a local percentage would divide by zero.

use crate::v2::core::ui::{badge_class, cn};
use crate::v2::core::utils::countdown::countdown_label;
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;
use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
/// The integer at `k`, or zero when the key is absent or not an integer.
fn vint(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
/// The boolean at `k`, or false when the key is absent or not a boolean.
fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// Badge variant for an `event_status` — the same table the admin event manager uses, so an
/// operation reads identically in the admin calendar and on the public schedule.
fn status_variant(status: &str) -> &'static str {
    match status {
        "open" => "success",
        "locked" => "warning",
        "live" => "primary",
        "completed" => "tertiary",
        "cancelled" => "error",
        _ => "neutral",
    }
}

/// One master-list operation card.
///
/// Built as a button rather than a list-detail item because the card carries a fill bar, and the
/// list-detail preview slot renders inside a `<p>` — a `<div>` bar there would be invalid nesting.
/// Row shape follows the admin event manager's day-ops row: title and mono meta line on the left,
/// status chips on the right.
pub(super) fn op_card(
    e: &Value,
    picked: RwSignal<Option<String>>,
    selected_id: Memo<Option<String>>,
) -> impl IntoView + use<> {
    let id = vstr(e, "id");
    let click_id = id.clone();
    let title = vstr(e, "name_override");
    let title = if title.is_empty() {
        "Untitled Operation".to_string()
    } else {
        title
    };
    let start = vstr(e, "start_time");
    let when = format_local_datetime(&start);
    let countdown = countdown_label(&start);
    let status = vstr(e, "status");
    let locked = vbool(e, "registration_locked");
    let missions = vint(e, "mission_count");
    let filled = vint(e, "filled");
    let total = vint(e, "total_slots");
    // `total_slots` is the materialized ORBAT slot count and is 0 until missions are attached, so
    // the bar is driven by the server's own `percent` and clamped — never a divide by zero.
    let pct = e
        .get("percent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    view! {
        <button
            type="button"
            on:click=move |_| picked.set(Some(click_id.clone()))
            class=move || {
                cn(
                    &[
                        "flex w-full flex-col gap-2 rounded-lg border p-3 text-left transition-all duration-200",
                        if selected_id.get().as_deref() == Some(id.as_str()) {
                            "border-primary/30 bg-surface-variant/80 shadow-[inset_0_0_15px_rgba(173,198,255,0.1)]"
                        } else {
                            "border-transparent hover:border-outline-variant/30 hover:bg-surface-variant/40"
                        },
                    ],
                )
            }
        >
            <div class="flex items-start justify-between gap-2">
                <span class="font-mono text-code-md text-primary opacity-80">{when}</span>
                <span class=badge_class(status_variant(&status))>{status.clone()}</span>
            </div>
            <h3 class="truncate font-semibold text-on-surface">{title}</h3>
            <div class="flex items-center justify-between gap-2 font-mono text-xs text-on-surface-variant">
                <span>
                    {missions} {if missions == 1 { " mission" } else { " missions" }} " · " {filled}
                    "/" {total} " slots"
                </span>
                <span class=if locked {
                    "text-tactical-yellow"
                } else {
                    "text-success"
                }>{if locked { "LOCKED".to_string() } else { countdown }}</span>
            </div>
            <div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-container-highest">
                <div
                    class="h-1.5 rounded-full bg-primary shadow-[0_0_10px_#adc6ff]"
                    style=format!("width: {pct}%;")
                ></div>
            </div>
        </button>
    }
}
