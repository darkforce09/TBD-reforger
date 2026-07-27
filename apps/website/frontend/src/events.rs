//! Event Schedule (/events) — ported from pages/operations.tsx `EventSchedulePage`. `<AuthGate>` →
//! `/events` Resource → `QueryState` → a `SplitPane` (upcoming-ops master list + an event-hub detail).
//!
//! **Empty-DB golden (unchanged):** with `Paginated` empty the master still shows "No upcoming
//! operations scheduled." and, with nothing to select, the detail still shows `SplitPaneEmpty` —
//! byte-exact-verified.
//!
//! **T-232:** the populated half was never written. The non-empty master branch was a literal
//! `().into_any()` and the detail was an *unconditional* `SplitPaneEmpty` with no selection state at
//! all — so against the live API (4 events) this page rendered an empty aside beside a permanently
//! empty pane, with no way to select anything. Now: op cards (local date, lifecycle + registration
//! badges, mission count, fill bar), the first one auto-selected per the surface spec
//! (`docs/website/frontend/pages/event-schedule.md` §Behavior step 2), and a real detail pane
//! driven by `GET /events/:id`.
//!
//! **Three states, not two (the T-226 lesson).** This is the one page in the T-232 set whose detail
//! needs its own fetch, so it is the one page that can go stale: a `LocalResource` keeps serving its
//! LAST value while the next run is in flight. Collapsing that into `Option<EventHub>` would fold
//! three different situations into one `None` — "nothing is selected", "the hub GET failed", and
//! "this hub belongs to the operation you were looking at BEFORE" — which renders as a spurious
//! error over a request about to succeed and, worse, puts one operation's `event_mission_id` under
//! another operation's chrome. [`Hub`] is therefore a three-variant enum that **carries the event
//! id it was loaded for**, and the detail renders the loading state whenever that id does not match
//! the current selection.
//!
//! **T-353:** detail column renders `event_hub::event_hub_view` (same body as `/events/:id`) so
//! inline ORBAT register matches the surface spec — no dossier summary + deep-link stand-in.
#![allow(dead_code)]
use crate::datefmt::{countdown_label, format_local_datetime};
use crate::dto::{EventHub, Paginated};
use crate::event_hub::event_hub_view;
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{badge_class, cn, AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
fn vint(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// Badge variant for an `event_status` — the same table `event_manager.rs` uses, so an operation
/// reads identically in the admin calendar and on the public schedule.
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

/// The detail pane's fetch state. See the module note: three states, and `Loaded` carries the id it
/// belongs to so a value left over from the previously-selected operation can never be rendered as
/// if it were the current one.
#[derive(Clone, PartialEq)]
enum Hub {
    /// Nothing selected (the empty schedule) — there is no hub to fetch, and any run still in
    /// flight must read as loading rather than as a failure.
    Idle,
    Failed,
    Loaded(String, EventHub),
}

#[component]
pub fn EventSchedulePage() -> impl IntoView {
    view! {
        <AuthGate>
            <EventScheduleInner />
        </AuthGate>
    }
}

#[component]
fn EventScheduleInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<Value>>(store, "/events")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                events
                    .get()
                    .map(|opt| match opt {
                        Some(page) => board(page.data).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn board(events: Vec<Value>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let events = StoredValue::new(events);
    // `None` = "the user has not picked yet", which resolves to the first row rather than to no
    // selection (spec §Behavior step 2). Derived instead of seeded through an `Effect` so there is
    // no write-during-render and no ordering question: clicking a card writes `picked`, everything
    // else reads `selected_id`.
    let picked = RwSignal::new(None::<String>);
    // The list is fixed for the life of this `board` call, so the fallback is computed once.
    let first_id = StoredValue::new(
        events.with_value(|e| e.first().map(|f| vstr(f, "id")).filter(|id| !id.is_empty())),
    );
    // A `Memo`, not a closure: the master rows, the hub Resource and the detail all read this, and
    // a `Memo` is `Copy + Send` (a captured generic closure is neither, which the reactive
    // `class=` attribute below requires) and recomputes once per change rather than per reader.
    let selected_id = Memo::new(move |_| picked.get().or_else(|| first_id.get_value()));

    let hub = LocalResource::new(move || {
        let id = selected_id.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match id {
                    Some(id) => {
                        match crate::client::api_get::<EventHub>(store, &format!("/events/{id}"))
                            .await
                        {
                            Ok(h) => Hub::Loaded(id, h),
                            Err(_) => Hub::Failed,
                        }
                    }
                    None => Hub::Idle,
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                Hub::Idle
            }
        }
    });

    let master_header = view! {
        <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Upcoming Ops"</h2>
    }
    .into_any();

    let master = view! {
        {move || {
            events
                .with_value(|events| {
                    if events.is_empty() {
                        return view! {
                            <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                "No upcoming operations scheduled."
                            </p>
                        }
                            .into_any();
                    }
                    events.iter().map(|e| op_card(e, picked, selected_id)).collect_view().into_any()
                })
        }}
    }
    .into_any();

    let detail = view! {
        {move || {
            let want = selected_id.get();
            match (want, hub.get()) {
                // Nothing to select: the empty-schedule resting state.
                (None, _) => {
                    view! {
                        <SplitPaneEmpty
                            icon=view! { <MaterialIcon name="calendar_month" class="text-4xl" /> }
                                .into_any()
                            message="Select an operation to view its hub."
                        />
                    }
                        .into_any()
                }
                // The value in hand is the one asked for — full inline-ORBAT hub (T-353).
                (Some(want), Some(Hub::Loaded(got, ev))) if got == want => {
                    let on_change = Callback::new(move |()| hub.refetch());
                    event_hub_view(ev, on_change).into_any()
                }
                (Some(_), Some(Hub::Failed)) => {
                    view! {
                        <div class="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
                            <MaterialIcon name="error" class="text-4xl text-error-alert" />
                            <p class="text-label-md text-on-surface-variant">
                                "Could not load this operation's hub."
                            </p>
                        </div>
                    }
                        .into_any()
                }
                // Everything else is in flight: the Resource is pending (`None`), it is `Idle`
                // because the run that will fetch `want` has not started, or it still holds the
                // PREVIOUS operation's hub. All three are "loading", never "empty" and never the
                // other operation's data.
                (Some(_), _) => {
                    view! {
                        <div class="flex h-full items-center justify-center">
                            <p class="text-label-md text-on-surface-variant">"Loading operation…"</p>
                        </div>
                    }
                        .into_any()
                }
            }
        }}
    }
    .into_any();

    view! {
        <SplitPane
            master_width="24rem"
            master_header=master_header
            master=master
            detail=detail
        />
    }
}

/// One master-list op card. Built as a button rather than a `ListDetailItem` because the spec's card
/// carries a fill bar, and `ListDetailItem`'s `preview` slot renders inside a `<p>` — a `<div>` bar
/// there would be invalid nesting. Row shape follows `event_manager.rs`'s day-ops row (the T-226
/// reference): title + mono meta line on the left, status chips on the right.
fn op_card(
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
