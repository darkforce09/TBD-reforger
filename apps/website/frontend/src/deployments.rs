//! My Deployments (/deployments) — ported from pages/operations.tsx `DeploymentsPage`. `<AuthGate>`
//! → `/deployments` Resource → `QueryState` → a two-pane service record: a left telemetry dossier
//! (identity from the auth store + real `total_operations` + an honest empty state until personal
//! telemetry counters land in T-397) and a right pane (Active Orders banner + Combat History +
//! Leave of Absence).
//!
//! **T-407 — personal mock telemetry is gone.** The left pane used to invent identical K/D,
//! win-rate, and favourite-loadout tiles for every operator beside the genuinely real
//! `total_operations`. Until T-397 ships NULL-honest personal stats, those tiles are an empty
//! state ("No telemetry recorded"), not invented numbers.
//!
//! **Empty-DB golden (unchanged):** with no upcoming and an empty history the "No Active Orders" +
//! "No Service History Compiled" states and the always-on dossier still render byte-for-byte as
//! before.
//!
//! **T-232:** both populated branches were literal `().into_any()`, so a caller with real
//! deployments (2 upcoming + 2 history rows against the live API) saw "Active Orders" over blank
//! space and an empty "Combat History" section. Both are now written: the Active-Orders banner
//! renders the caller's next deployment (countdown, assigned slot, ORBAT deep link) with the rest of
//! the queue beneath it, and Combat History is the service-record table the surface spec asks for
//! (Date / Operation / Role / Outcome / AAR).
//!
//! Neither branch fetches anything — `GET /me/deployments` returns both lists in one payload — so
//! there is no second Resource here to go stale (the T-226 hazard). Items stay `serde_json::Value`;
//! the fields read are pinned by the `registration_state` and `mission_outcome` enums.
//!
//! **T-265 — Leave of Absence.** The LOA backend (`POST/GET /me/leave-requests`,
//! `GET/PATCH /admin/leave-requests`) had zero SPA surface. Member file+list and (for admins) the
//! review queue live as sections on this page — no new route / nav entry (surface spec item 8).
//! Each LOA panel owns its own `LocalResource` so a submit/review refetch cannot go stale against
//! the deployments payload.
#![allow(dead_code)]
use crate::core::auth::AuthStore;
use crate::core::datefmt::{countdown_label, format_local_datetime, format_short_date};
use crate::core::dto::{CreateLeaveInput, DataEnvelope, Deployments, LeaveRequest, Paginated};
use crate::core::ui::{badge_class, MaterialIcon};
use crate::shell::nav_config::Role;
// T-405 — the AAR `<a href>` at the bottom of Combat History is the sink of the T-391 XSS.
use crate::core::url_guard;
use leptos::prelude::*;
use serde_json::Value;

/// Form control recipe shared with CreateMissionDialog (macOS pill).
const LOA_INPUT: &str = "w-full rounded-full bg-white/5 px-5 py-2.5 font-mono text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none transition focus:ring-1 focus:ring-primary/50";

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// `registration_state` → its `badge_class` variant.
fn state_variant(state: &str) -> &'static str {
    match state {
        "registered" | "attended" => "success",
        "waitlisted" => "warning",
        "withdrawn" | "no_show" => "error",
        _ => "neutral",
    }
}

/// `mission_outcome` → the service-record label the spec names ("MISSION SUCCESS / FAILED").
fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "success" => "Mission Success",
        "failure" => "Failed",
        "aborted" => "Aborted",
        "pending" => "Pending",
        _ => "Unknown",
    }
}

fn outcome_variant(outcome: &str) -> &'static str {
    match outcome {
        "success" => "success",
        "failure" => "error",
        "aborted" => "warning",
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

/// Client-side mirror of `handlers::deployments::submit_leave` date rules. Bare `YYYY-MM-DD`
/// only — the response wire form (`…T00:00:00Z`) must never be posted back.
fn validate_loa_range(starts_on: &str, ends_on: &str) -> Result<(), &'static str> {
    if starts_on.is_empty() || ends_on.is_empty() {
        return Err("starts_on and ends_on are required");
    }
    if !is_ymd(starts_on) || !is_ymd(ends_on) {
        return Err("dates must be YYYY-MM-DD");
    }
    if ends_on < starts_on {
        return Err("ends_on must be on or after starts_on");
    }
    Ok(())
}

fn is_ymd(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn leave_status_variant(status: &str) -> &'static str {
    match status {
        "approved" => "success",
        "denied" => "error",
        "pending" => "warning",
        _ => "neutral",
    }
}

/// Serialize the create body the form POSTs — kept as a named helper so the DTO is exercised on
/// the native test target (the submit closure itself is `cfg(wasm32)`).
fn create_leave_body(starts_on: String, ends_on: String, reason: String) -> Value {
    serde_json::to_value(CreateLeaveInput {
        starts_on,
        ends_on,
        reason,
    })
    .unwrap_or(Value::Null)
}

/// "BLUFOR · Command · Platoon Leader" from whichever of `faction` / `squad` / `role` the backend
/// actually filled — an unassigned registration carries none of the three, and joining blindly would
/// render a row of bare separators.
fn slot_line(u: &Value) -> String {
    ["faction", "squad", "role"]
        .iter()
        .map(|k| vstr(u, k))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Affordance shown where personal K/D / win-rate / fav-loadout once pretended to be live.
/// Kept as a named constant so Class-R can pin the empty copy without driving a `view!`.
const NO_TELEMETRY_RECORDED: &str = "No telemetry recorded";

const BANNER_IMG: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'><rect width='400' height='200' fill='%23151b2b'/><g stroke='%23adc6ff' stroke-width='0.5' opacity='0.5'><path d='M0 40 H400 M0 80 H400 M0 120 H400 M0 160 H400 M50 0 V200 M120 0 V200 M190 0 V200 M260 0 V200 M330 0 V200'/></g><circle cx='190' cy='100' r='26' fill='none' stroke='%23facc15' stroke-width='1.5'/><path d='M190 66 V134 M156 100 H224' stroke='%23facc15' stroke-width='1'/></svg>";

#[component]
pub fn DeploymentsPage() -> impl IntoView {
    view! {
        <crate::core::ui::AuthGate>
            <DeploymentsInner />
        </crate::core::ui::AuthGate>
    }
}

#[component]
fn DeploymentsInner() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let data = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<Deployments>(store, "/me/deployments")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Deployments>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                data.get()
                    .map(|opt| match opt {
                        Some(d) => dossier(d).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn dossier(d: Deployments) -> impl IntoView {
    let user = expect_context::<AuthStore>().user.get();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let role = user.as_ref().map(|u| u.role.as_str()).unwrap_or_default();
    let is_admin = user.as_ref().is_some_and(|u| matches!(u.role, Role::Admin));
    let has_active = !d.upcoming.is_empty();
    let has_history = !d.service_history.is_empty();
    let upcoming = d.upcoming.clone();
    let history = d.service_history.clone();

    view! {
        <div class="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
            <div class="flex h-full w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl lg:flex-row">
                // ── Left: telemetry dossier ──
                <aside class="custom-scrollbar flex shrink-0 flex-col gap-8 overflow-y-auto border-b border-white/10 bg-surface-container-lowest/40 p-8 lg:w-[30%] lg:border-b-0 lg:border-r">
                    <header>
                        <div class="mb-6 flex h-16 w-16 items-center justify-center text-primary">
                            <MaterialIcon name="military_tech" class="text-[4rem] leading-none" />
                        </div>
                        <h2 class="text-4xl font-black uppercase leading-none tracking-tighter text-on-surface">
                            {username}
                        </h2>
                        <span class="mt-1 block font-mono text-sm uppercase tracking-widest text-primary">
                            {role}
                        </span>
                    </header>
                    <div class="space-y-5">
                        <div>
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Total Deployments"
                            </span>
                            <p class="font-mono text-3xl font-bold text-on-surface">
                                {d.total_operations}
                            </p>
                        </div>
                        <div class="border-t border-white/10 pt-6">
                            <span class="mb-2 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Personal Telemetry"
                            </span>
                            <p class="font-mono text-sm text-on-surface-variant">
                                {NO_TELEMETRY_RECORDED}
                            </p>
                        </div>
                    </div>
                </aside>

                // ── Right: active orders + combat history + LOA ──
                <main class="custom-scrollbar flex min-h-0 flex-1 flex-col overflow-y-auto bg-surface-container-highest/10">
                    <section class="relative shrink-0 overflow-hidden border-b border-white/10">
                        <img
                            src=BANNER_IMG
                            alt=""
                            class="absolute inset-0 h-full w-full object-cover opacity-30 mix-blend-luminosity"
                        />
                        <div class="absolute inset-0 bg-gradient-to-r from-surface-container-lowest/80 to-transparent"></div>
                        <div class="relative z-10 flex min-h-[240px] flex-col justify-center gap-3 p-8">
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Active Orders"
                            </span>
                            {if has_active {
                                active_orders(upcoming).into_any()
                            } else {
                                view! {
                                    <div class="flex flex-col items-center justify-center gap-3 py-4 text-center">
                                        <MaterialIcon
                                            name="track_changes"
                                            class="text-7xl text-on-surface-variant/40 animate-pulse drop-shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                                        />
                                        <h3 class="text-3xl font-black uppercase tracking-tight text-on-surface-variant/60">
                                            "No Active Orders"
                                        </h3>
                                        <p class="font-mono text-sm text-on-surface-variant">
                                            "Stand by for deployment tasking."
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }}
                        </div>
                    </section>
                    <section class="p-8">
                        <h2 class="mb-4 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                            "Combat History"
                        </h2>
                        {if has_history {
                            service_record(history).into_any()
                        } else {
                            view! {
                                <div class="bg-grid-overlay flex min-h-[200px] items-center justify-center rounded-xl border border-white/10 shadow-[inset_0_0_30px_rgba(173,198,255,0.06)]">
                                    <p class="font-mono text-code-md uppercase tracking-widest text-on-surface-variant">
                                        "No Service History Compiled"
                                    </p>
                                </div>
                            }
                                .into_any()
                        }}
                    </section>
                    <LeaveOfAbsencePanel />
                    {is_admin.then(|| {
                        view! { <AdminLeaveQueue /> }
                    })}
                </main>
            </div>
        </div>
    }
}

/// The Active Orders banner. `upcoming` is ordered soonest-first by the backend, so `[0]` is the
/// hero order and the rest queue beneath it — the spec's "Awaiting Deployment" list without a
/// second heading, since the banner already carries one.
fn active_orders(upcoming: Vec<Value>) -> impl IntoView {
    let mut it = upcoming.into_iter();
    let Some(next) = it.next() else {
        // `has_active` is checked by the caller; this arm exists so the function is total.
        return ().into_any();
    };
    let rest: Vec<Value> = it.collect();
    let name = vstr(&next, "name");
    let name = if name.is_empty() {
        "Untitled Operation".to_string()
    } else {
        name
    };
    let start = vstr(&next, "start_time");
    let when = format_local_datetime(&start);
    let countdown = countdown_label(&start);
    let terrain = terrain_label(&vstr(&next, "terrain"));
    let state = vstr(&next, "state");
    let slot = slot_line(&next);
    let event_id = vstr(&next, "event_id");
    let emid = vstr(&next, "event_mission_id");
    // Item 9 of the surface spec — `/events/:id/missions/:emid/orbat`, the route `router.rs:105`
    // registers. Rendered only when both ids are on the wire; a registration with no
    // `event_mission_id` has no ORBAT to modify.
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
                            view! { <span class=badge_class(state_variant(&state))>{state.clone()}</span> }
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
                                    let n = vstr(&u, "name");
                                    let n = if n.is_empty() {
                                        "Untitled Operation".to_string()
                                    } else {
                                        n
                                    };
                                    let s = vstr(&u, "start_time");
                                    let st = vstr(&u, "state");
                                    let eid = vstr(&u, "event_id");
                                    let row = view! {
                                        <>
                                            <span class="min-w-0 flex-1 truncate text-on-surface">{n}</span>
                                            <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                {format_local_datetime(&s)}
                                            </span>
                                            {(!st.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <span class=badge_class(state_variant(&st))>{st.clone()}</span>
                                                    }
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

/// The Service Record table — spec items 11–18. A real `<table>` rather than a div grid: it is
/// tabular data, and the wide-table roster (`personnel.rs`) is the platform precedent for one.
fn service_record(history: Vec<Value>) -> impl IntoView {
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[40rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Date" />
                        <ServiceHead label="Operation" />
                        <ServiceHead label="Role Played" />
                        <ServiceHead label="Outcome" />
                        <ServiceHead label="AAR" />
                    </tr>
                </thead>
                <tbody>
                    {history
                        .into_iter()
                        .map(|h| {
                            let date = vstr(&h, "date");
                            let operation = vstr(&h, "operation");
                            let operation = if operation.is_empty() {
                                "—".to_string()
                            } else {
                                operation
                            };
                            let role = vstr(&h, "role");
                            let role = if role.is_empty() { "—".to_string() } else { role };
                            let outcome = vstr(&h, "outcome");
                            let replay = vstr(&h, "aar_replay_url");
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&date)}
                                    </td>
                                    <td class="px-4 py-3 font-medium text-on-surface">{operation}</td>
                                    <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                                        {role}
                                    </td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(
                                            outcome_variant(&outcome),
                                        )>{outcome_label(&outcome)}</span>
                                    </td>
                                    <td class="px-4 py-3">
                                        {match replay_href(&replay) {
                                            None => {
                                                view! {
                                                    <span class="font-mono text-xs text-outline">"—"</span>
                                                }
                                                    .into_any()
                                            }
                                            Some(href) => {
                                                view! {
                                                    <a
                                                        href=href
                                                        target="_blank"
                                                        rel="noreferrer"
                                                        class="inline-flex items-center gap-1 font-mono text-xs tracking-wider text-primary uppercase transition hover:underline"
                                                    >
                                                        <MaterialIcon name="play_circle" class="text-base" />
                                                        "View Replay"
                                                    </a>
                                                }
                                                    .into_any()
                                            }
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// The AAR cell's one decision: is this stored string safe to put in an `href`, or does the row
/// get the inert em-dash? **T-405 — the sink half of the T-391 XSS.**
///
/// Before this existed the cell bound `href=replay` after testing only `replay.is_empty()`, so a
/// `javascript:` URL stored before T-391's write guard shipped executed on click. The
/// `rel="noreferrer"` already on that anchor was never a mitigation — it governs the `Referer`
/// header, not what the scheme does. Neither is HTML escaping: a `javascript:` href is not a quote
/// breakout, it is a well-formed attribute whose *content* runs, so the only safe move at a sink
/// is to not emit the attribute at all.
///
/// **The empty case is unchanged, not merely preserved by accident.** `""` carries no scheme, so
/// [`url_guard::is_http_url`] answers `false` for it and the cell takes the same em-dash branch it
/// always did. The new test strictly subsumes the old one.
///
/// This duplicates the API's write-boundary check on purpose, and the duplication is not waste:
/// the write guard governs values that arrived through `upsert_match` *after* it shipped, and
/// this governs every value that reaches this table whatever door it came in by — a pre-guard
/// row, an operator's `psql`, a writer somebody adds later. They fail independently, which is the
/// entire point of guarding an output.
///
/// Extracted from the `view!` rather than left inline so it can be tested: the crate is CSR-only
/// and cannot render to a string natively, so a returned `Option` is the largest testable unit
/// this cell has.
fn replay_href(replay: &str) -> Option<&str> {
    url_guard::is_http_url(replay).then_some(replay)
}

/* ─────────────────────────── T-265 Leave of Absence ─────────────────────────── */

/// Member LOA: file a request (`POST /me/leave-requests`) + list own rows
/// (`GET /me/leave-requests`). Surface-spec item 8 — lives on `/deployments`, no new route.
#[component]
fn LeaveOfAbsencePanel() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let mine = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<DataEnvelope<LeaveRequest>>(store, "/me/leave-requests")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<LeaveRequest>>
        }
    });
    let starts_on = RwSignal::new(String::new());
    let ends_on = RwSignal::new(String::new());
    let reason = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let form_error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::core::toast::use_toasts();
            let start = starts_on.get_untracked().trim().to_string();
            let end = ends_on.get_untracked().trim().to_string();
            let why = reason.get_untracked().trim().to_string();
            if let Err(msg) = validate_loa_range(&start, &end) {
                form_error.set(Some(msg.to_string()));
                toasts.error(msg);
                return;
            }
            form_error.set(None);
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let body = create_leave_body(start, end, why);
            leptos::task::spawn_local(async move {
                match crate::core::client::api_post::<LeaveRequest>(
                    store,
                    "/me/leave-requests",
                    body,
                )
                .await
                {
                    Ok(_) => {
                        toasts.success("Leave request submitted");
                        starts_on.set(String::new());
                        ends_on.set(String::new());
                        reason.set(String::new());
                        mine.refetch();
                    }
                    Err(e) => toasts.error(crate::core::client::api_error_message(
                        &e,
                        "Failed to submit leave request",
                    )),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <section class="border-t border-white/10 p-8">
            <div class="mb-4 flex flex-wrap items-baseline justify-between gap-2">
                <h2 class="font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "Leave of Absence"
                </h2>
                <span class="font-mono text-[10px] tracking-widest text-on-surface-variant/70 uppercase">
                    "Submit Leave of Absence"
                </span>
            </div>
            <form on:submit=on_submit class="mb-6 grid gap-4 md:grid-cols-4">
                <div>
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Starts on"
                    </label>
                    <input
                        type="date"
                        required
                        prop:value=move || starts_on.get()
                        on:input=move |ev| starts_on.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div>
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Ends on"
                    </label>
                    <input
                        type="date"
                        required
                        prop:value=move || ends_on.get()
                        on:input=move |ev| ends_on.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div class="md:col-span-2">
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Reason"
                    </label>
                    <input
                        type="text"
                        placeholder="Optional reason…"
                        prop:value=move || reason.get()
                        on:input=move |ev| reason.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div class="md:col-span-4 flex flex-wrap items-center gap-3">
                    <button
                        type="submit"
                        prop:disabled=move || busy.get()
                        class="inline-flex items-center gap-2 rounded-full border border-primary/50 bg-primary/15 px-5 py-2.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/25 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                        <MaterialIcon name="event_busy" class="text-base" />
                        {move || {
                            if busy.get() { "Submitting…" } else { "Submit Leave of Absence" }
                        }}
                    </button>
                    {move || {
                        form_error
                            .get()
                            .map(|e| {
                                view! {
                                    <span class="font-mono text-xs text-error">{e}</span>
                                }
                            })
                    }}
                </div>
            </form>
            <Suspense fallback=move || {
                view! {
                    <p class="font-mono text-xs text-on-surface-variant">"Loading leave requests…"</p>
                }
            }>
                {move || {
                    mine.get().map(|opt| match opt {
                        Some(env) => leave_rows(&env.data).into_any(),
                        None => {
                            view! {
                                <p class="font-mono text-xs text-error">
                                    "Failed to load leave requests."
                                </p>
                            }
                                .into_any()
                        }
                    })
                }}
            </Suspense>
        </section>
    }
}

/// Admin review queue on the same page (admin role only). Prefer this over a new
/// `/admin/leave-requests` route — `personnel.rs` / router are outside owns.
#[component]
fn AdminLeaveQueue() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let queue = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<Paginated<LeaveRequest>>(store, "/admin/leave-requests")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<LeaveRequest>>
        }
    });

    view! {
        <section class="border-t border-white/10 p-8">
            <div class="mb-4 flex flex-wrap items-baseline justify-between gap-2">
                <h2 class="font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "LOA Review Queue"
                </h2>
                <span class="font-mono text-[10px] tracking-widest text-tactical-yellow/80 uppercase">
                    "Admin"
                </span>
            </div>
            <Suspense fallback=move || {
                view! {
                    <p class="font-mono text-xs text-on-surface-variant">"Loading review queue…"</p>
                }
            }>
                {move || {
                    queue.get().map(|opt| match opt {
                        Some(page) => admin_leave_table(page.data, queue).into_any(),
                        None => {
                            view! {
                                <p class="font-mono text-xs text-error">
                                    "Failed to load LOA review queue."
                                </p>
                            }
                                .into_any()
                        }
                    })
                }}
            </Suspense>
        </section>
    }
}

fn leave_rows(rows: &[LeaveRequest]) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <div class="rounded-xl border border-white/10 px-4 py-6 text-center">
                <p class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                    "No leave requests on file"
                </p>
            </div>
        }
        .into_any();
    }
    let rows = rows.to_vec();
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[36rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Starts" />
                        <ServiceHead label="Ends" />
                        <ServiceHead label="Reason" />
                        <ServiceHead label="Status" />
                        <ServiceHead label="Filed" />
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            let status_label = r.status.clone();
                            let variant = leave_status_variant(&r.status);
                            let reason = if r.reason.is_empty() {
                                "—".to_string()
                            } else {
                                r.reason.clone()
                            };
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.starts_on)}
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.ends_on)}
                                    </td>
                                    <td class="px-4 py-3 text-on-surface">{reason}</td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(variant)>{status_label}</span>
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.created_at)}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

fn admin_leave_table(
    rows: Vec<LeaveRequest>,
    queue: LocalResource<Option<Paginated<LeaveRequest>>>,
) -> impl IntoView {
    let store = expect_context::<AuthStore>();
    if rows.is_empty() {
        return view! {
            <div class="rounded-xl border border-white/10 px-4 py-6 text-center">
                <p class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                    "No leave requests in the queue"
                </p>
            </div>
        }
        .into_any();
    }
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[44rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Member" />
                        <ServiceHead label="Starts" />
                        <ServiceHead label="Ends" />
                        <ServiceHead label="Reason" />
                        <ServiceHead label="Status" />
                        <ServiceHead label="Review" />
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            let id = r.id.clone();
                            let id_deny = r.id.clone();
                            let status_label = r.status.clone();
                            let variant = leave_status_variant(&r.status);
                            let pending = r.status == "pending";
                            let reason = if r.reason.is_empty() {
                                "—".to_string()
                            } else {
                                r.reason.clone()
                            };
                            let on_approve = move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let toasts = crate::core::toast::use_toasts();
                                    let path = format!("/admin/leave-requests/{id}");
                                    leptos::task::spawn_local(async move {
                                        match crate::core::client::api_patch::<Value>(
                                            store,
                                            &path,
                                            serde_json::json!({"status":"approved"}),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                toasts.success("LOA approved");
                                                queue.refetch();
                                            }
                                            Err(e) => toasts.error(
                                                crate::core::client::api_error_message(
                                                    &e,
                                                    "Failed to approve LOA",
                                                ),
                                            ),
                                        }
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (&store, &queue, &id);
                                }
                            };
                            let on_deny = move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let toasts = crate::core::toast::use_toasts();
                                    let path = format!("/admin/leave-requests/{id_deny}");
                                    leptos::task::spawn_local(async move {
                                        match crate::core::client::api_patch::<Value>(
                                            store,
                                            &path,
                                            serde_json::json!({"status":"denied"}),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                toasts.success("LOA denied");
                                                queue.refetch();
                                            }
                                            Err(e) => toasts.error(
                                                crate::core::client::api_error_message(
                                                    &e,
                                                    "Failed to deny LOA",
                                                ),
                                            ),
                                        }
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (&store, &queue, &id_deny);
                                }
                            };
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                                        {r.discord_id.clone()}
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.starts_on)}
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.ends_on)}
                                    </td>
                                    <td class="px-4 py-3 text-on-surface">{reason}</td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(variant)>{status_label}</span>
                                    </td>
                                    <td class="px-4 py-3">
                                        {if pending {
                                            view! {
                                                <div class="flex flex-wrap gap-2">
                                                    <button
                                                        type="button"
                                                        on:click=on_approve
                                                        class="rounded-full bg-emerald-600/90 px-3 py-1.5 font-mono text-[10px] tracking-widest text-white uppercase transition hover:bg-emerald-500"
                                                    >
                                                        "Approve"
                                                    </button>
                                                    <button
                                                        type="button"
                                                        on:click=on_deny
                                                        class="rounded-full border border-error/40 bg-error/10 px-3 py-1.5 font-mono text-[10px] tracking-widest text-error uppercase transition hover:bg-error/20"
                                                    >
                                                        "Deny"
                                                    </button>
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <span class="font-mono text-xs text-outline">
                                                    {r.reviewed_by
                                                        .clone()
                                                        .unwrap_or_else(|| "—".into())}
                                                </span>
                                            }
                                                .into_any()
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn ServiceHead(label: &'static str) -> impl IntoView {
    view! {
        <th class="px-4 py-3 font-mono text-[10px] font-normal tracking-widest text-on-surface-variant uppercase">
            {label}
        </th>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The same table both `is_http_url` implementations are pinned to — reused here so the CELL is
    // checked against the adversarial corpus, not just the predicate underneath it. If a future
    // edit reverts this cell to `!replay.is_empty()`, every `false` row stops returning `None` and
    // the test below names the exact payload that would have rendered.
    include!("../../shared/is_http_url_cases.rs");

    #[test]
    fn aar_cell_emits_an_href_only_for_http_urls() {
        let mut wrong = Vec::new();
        for (input, should_link) in IS_HTTP_URL_CASES {
            match (replay_href(input), should_link) {
                (Some(_), false) => wrong.push(format!("  RENDERED AN HREF FOR {input:?}")),
                (None, true) => wrong.push(format!("  refused a legitimate link {input:?}")),
                _ => {}
            }
        }
        assert!(
            wrong.is_empty(),
            "the AAR replay cell is wrong on {} of {} cases:\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
    }

    /// The specific regression, spelled out rather than left implicit in the table sweep: the
    /// literal T-391 payload must produce no anchor, and the empty case must keep behaving
    /// exactly as it did before T-405 touched this cell.
    #[test]
    fn the_t391_payload_renders_no_anchor_and_empty_still_means_no_link() {
        assert_eq!(replay_href("javascript:alert(1)"), None);
        assert_eq!(replay_href("JaVaScRiPt:alert(1)"), None);
        assert_eq!(replay_href("java\tscript:alert(1)"), None);
        assert_eq!(
            replay_href("data:text/html,<script>alert(1)</script>"),
            None
        );
        // Unchanged from before the guard: no replay uploaded yet renders the em-dash.
        assert_eq!(replay_href(""), None);
        // ...and a real replay link still renders, which is the half that keeps the guard alive.
        assert_eq!(
            replay_href("https://aar.tbd/replays/abc.json"),
            Some("https://aar.tbd/replays/abc.json")
        );
    }

    /// Mirrors `submit_leave` — empty, non-YMD, and inverted ranges must fail before the POST.
    #[test]
    fn loa_date_validation_matches_backend_rules() {
        assert_eq!(
            validate_loa_range("", "2026-08-05").unwrap_err(),
            "starts_on and ends_on are required"
        );
        assert_eq!(
            validate_loa_range("nope", "2026-08-05").unwrap_err(),
            "dates must be YYYY-MM-DD"
        );
        assert_eq!(
            validate_loa_range("2026-08-01T00:00:00Z", "2026-08-05").unwrap_err(),
            "dates must be YYYY-MM-DD"
        );
        assert_eq!(
            validate_loa_range("2026-08-05", "2026-08-01").unwrap_err(),
            "ends_on must be on or after starts_on"
        );
        assert!(validate_loa_range("2026-08-01", "2026-08-01").is_ok());
        assert!(validate_loa_range("2026-08-01", "2026-08-05").is_ok());
    }

    #[test]
    fn create_leave_body_is_bare_ymd_json() {
        let v = create_leave_body("2026-08-01".into(), "2026-08-05".into(), "holiday".into());
        assert_eq!(
            v,
            serde_json::json!({
                "starts_on": "2026-08-01",
                "ends_on": "2026-08-05",
                "reason": "holiday",
            })
        );
    }

    #[test]
    fn leave_status_badge_variants() {
        assert_eq!(leave_status_variant("pending"), "warning");
        assert_eq!(leave_status_variant("approved"), "success");
        assert_eq!(leave_status_variant("denied"), "error");
        assert_eq!(leave_status_variant("bogus"), "neutral");
    }

    /// T-407 — personal K/D / win-rate / fav-loadout were hardcoded beside real
    /// `total_operations`. If any of these needles return, every operator sees the same
    /// fabricated stats again. Needles are `concat!`-split so this test does not match itself.
    /// Do not restate the banned literals in comments above — paraphrase, or this goes red.
    #[test]
    fn no_fabricated_personal_telemetry_survives_in_this_module() {
        const SRC: &str = include_str!("deployments.rs");
        let banned = [
            concat!("MOCK_", "KD"),
            concat!("MOCK_", "WIN_RATE"),
            concat!("FAV_", "WEAPON_NAME"),
            concat!("FAV_", "ASSET_NAME"),
            concat!("FAV_", "WEAPON_IMG"),
            concat!("FAV_", "ASSET_IMG"),
            concat!("2.", "45"),
            concat!("68", "%"),
            concat!("M4A1 ", "Block II"),
            concat!("M1A2 ", "Abrams"),
            concat!("Telemetry", "Stat"),
            concat!("Fav", "Loadout"),
        ];
        for needle in banned {
            assert!(
                !SRC.contains(needle),
                "fabricated personal telemetry is back in deployments.rs: {needle:?}. \
                 Until T-397, show the empty affordance — never invent numbers."
            );
        }
        assert!(
            SRC.contains(NO_TELEMETRY_RECORDED),
            "the honest empty affordance must stay on the page"
        );
    }

    #[test]
    fn personal_telemetry_empty_copy_is_pinned() {
        assert_eq!(NO_TELEMETRY_RECORDED, "No telemetry recorded");
    }
}
