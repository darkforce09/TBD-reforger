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
//! **Scope note:** the spec's full inline-ORBAT detail is `event_hub.rs`'s `event_hub_view`, which
//! is private to that module (T-232 owns neither it nor `ui.rs`). The detail here is the
//! self-contained dossier summary plus a deep link to `/events/:id`; exporting `event_hub_view` is
//! reported as the follow-up.
#![allow(dead_code)]
use crate::datefmt::{countdown_label, format_local_datetime};
use crate::dto::{EventHub, EventMissionDossier, Paginated};
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{badge_class, cn, AuthGate, MaterialIcon};
use crate::url_guard;
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

fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "—".into(),
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
                // The value in hand is the one asked for.
                (Some(want), Some(Hub::Loaded(got, ev))) if got == want => {
                    hub_detail(ev).into_any()
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

/// The detail column: the operation hero + its attached mission dossiers. The full inline-ORBAT hub
/// lives in `event_hub.rs` (see the module note) and is reachable from the header link.
fn hub_detail(ev: EventHub) -> impl IntoView {
    let title = ev
        .name_override
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Untitled Operation".into());
    let when = format_local_datetime(&ev.start_time);
    let countdown = countdown_label(&ev.start_time);
    let href = format!("/events/{}", ev.id);
    let mission_count = ev.missions.len();
    let banner = ev.banner_image_url.clone().unwrap_or_default();
    let briefing = ev.briefing.clone().unwrap_or_default();
    let status = ev.status.clone();
    let status_class = badge_class(status_variant(&status));
    let locked = ev.registration_locked;
    let max_slots = ev.max_slots;
    view! {
        <div class="flex flex-col">
            <header class="relative overflow-hidden border-b border-outline-variant/30">
                {banner_img_src(&banner)
                    .map(|src| {
                        view! {
                            <>
                                <img
                                    src=src.to_string()
                                    alt=""
                                    class="absolute inset-0 h-full w-full object-cover opacity-25 mix-blend-luminosity"
                                />
                                <div class="absolute inset-0 bg-gradient-to-r from-surface-container-lowest/80 to-transparent"></div>
                            </>
                        }
                    })}
                <div class="relative z-10 flex flex-col gap-4 px-8 py-8">
                    <div class="flex flex-wrap items-center gap-2">
                        <span class=status_class>{status}</span>
                        <span class=badge_class(
                            if locked { "neutral" } else { "success" },
                        )>{if locked { "Registration locked" } else { "Registration open" }}</span>
                    </div>
                    <h1 class="text-headline-md tracking-tight text-on-surface">{title}</h1>
                    <div class="flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                        <span>{when}</span>
                        <span class="text-primary">"T-MINUS "{countdown}</span>
                        {(max_slots > 0)
                            .then(|| {
                                view! { <span>{max_slots}" slot cap"</span> }
                            })}
                    </div>
                    <a
                        href=href
                        class="group inline-flex w-fit items-center gap-2 rounded-lg border border-primary/50 bg-surface/50 px-5 py-2.5 text-sm font-bold tracking-widest text-primary uppercase backdrop-blur-md transition-all hover:bg-primary/20 active:scale-95"
                    >
                        "Open Operation Hub"
                        <MaterialIcon
                            name="arrow_forward"
                            class="transition-transform group-hover:translate-x-1"
                        />
                    </a>
                </div>
            </header>
            {(!briefing.is_empty())
                .then(|| {
                    view! {
                        <section class="border-b border-outline-variant/30 px-8 py-6">
                            <h2 class="mb-3 font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                                "Briefing"
                            </h2>
                            <p class="max-w-prose whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                                {briefing}
                            </p>
                        </section>
                    }
                })}
            <section class="px-8 py-6">
                <h2 class="mb-4 font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                    "Attached Missions "
                    <span class="text-on-surface">"("{mission_count}")"</span>
                </h2>
                {if ev.missions.is_empty() {
                    view! {
                        <p class="text-label-md text-on-surface-variant">
                            "No missions attached to this operation yet."
                        </p>
                    }
                        .into_any()
                } else {
                    let event_id = ev.id.clone();
                    view! {
                        <div class="flex flex-col gap-3">
                            {ev
                                .missions
                                .into_iter()
                                .map(|m| mission_row(&event_id, m))
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }}
            </section>
        </div>
    }
}

/// One attached-mission summary: title, terrain / mode, fill, factions, and the caller's own
/// registration state when the backend reported one.
///
/// The ORBAT deep link is `/events/:id/missions/:emid/orbat` — the shape `router.rs:105` actually
/// registers. The dossier only carries `event_mission_id`, so the event id is threaded in from the
/// hub: building it from the dossier alone would produce a route that does not exist.
fn mission_row(event_id: &str, m: EventMissionDossier) -> impl IntoView + use<> {
    let orbat_href = format!("/events/{event_id}/missions/{}/orbat", m.event_mission_id);
    let factions = if m.factions.is_empty() {
        String::new()
    } else {
        m.factions.join(" · ")
    };
    let my_state = m.my_state.clone().unwrap_or_default();
    view! {
        <div class="rounded-xl border border-white/10 bg-surface-container-lowest/40 p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                    <h3 class="truncate font-semibold text-on-surface">{m.title}</h3>
                    <p class="mt-1 font-mono text-xs text-on-surface-variant">
                        {terrain_label(&m.terrain)} " · " {m.game_mode.clone()} " · " {m.filled} "/"
                        {m.total} " slots"
                    </p>
                    {(!factions.is_empty())
                        .then(|| {
                            view! {
                                <p class="mt-1 font-mono text-xs text-outline">{factions}</p>
                            }
                        })}
                </div>
                <div class="flex shrink-0 items-center gap-2">
                    {(!my_state.is_empty())
                        .then(|| {
                            let variant = match my_state.as_str() {
                                "registered" | "attended" => "success",
                                "waitlisted" => "warning",
                                "no_show" => "error",
                                _ => "neutral",
                            };
                            view! { <span class=badge_class(variant)>{my_state.clone()}</span> }
                        })}
                    <a
                        href=orbat_href
                        class="rounded-lg border border-white/10 px-3 py-1.5 font-mono text-xs tracking-wider text-on-surface uppercase transition hover:bg-white/5"
                    >
                        "ORBAT"
                    </a>
                </div>
            </div>
        </div>
    }
}

/// Event hub banner `<img src>`. **T-413** — empty / non-http → no banner (same empty state as before).
fn banner_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::banner_img_src;

    include!("../../shared/is_http_url_cases.rs");

    #[test]
    fn event_banner_emits_src_only_for_http_urls() {
        let mut wrong = Vec::new();
        for (input, should_img) in IS_HTTP_URL_CASES {
            match (banner_img_src(input), should_img) {
                (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
                (None, true) => wrong.push(format!("  refused a legitimate banner {input:?}")),
                _ => {}
            }
        }
        assert!(
            wrong.is_empty(),
            "event banner sink wrong on {} of {} cases:\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
    }
}
