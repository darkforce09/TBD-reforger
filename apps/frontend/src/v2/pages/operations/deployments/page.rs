//! The caller's own service record: what they are slotted into, and what they have flown.
//!
//! **Role:** fetches the caller's deployments and lays the page out — the identity and
//! deployment-count column on the left, and on the right the active-orders banner, the combat
//! history table, the leave-of-absence panel and, for an administrator, the leave review queue.
//! **Position:** the `/deployments` route, rendered inside the navigation frame behind the
//! sign-in gate.
//! **Signals & state:** reads the caller's name and tier, which also decide administrator status,
//! from the session store through a memo, so a profile poll that changes neither does not rebuild
//! the record. Owns the one resource; both lists arrive in the same payload, so nothing on this
//! page can go stale against anything else on it.
//! **Invariants:** the personal-telemetry block is an explicit empty affordance, not invented
//! numbers — the only genuinely served figure is the total deployment count. The fetch is a
//! browser-only path and resolves to `None` in a native build.
#![allow(dead_code)]

use super::active_orders::active_orders;
use super::leave_of_absence::LeaveOfAbsencePanel;
use super::leave_review_queue::AdminLeaveQueue;
use super::service_record::service_record;
use crate::v2::core::api::dto::Deployments;
use crate::v2::core::auth::AuthStore;
use crate::v2::core::auth::Role;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// The copy shown where personal telemetry would go.
///
/// A named constant rather than a literal inside the view, so the empty copy can be pinned
/// without rendering: the crate is browser-only and a view cannot be exercised natively.
pub(super) const NO_TELEMETRY_RECORDED: &str = "No telemetry recorded";

/// The banner artwork behind the active-orders section: a grid and a reticle, inline so the
/// page always has something to draw.
const BANNER_IMG: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'><rect width='400' height='200' fill='%23151b2b'/><g stroke='%23adc6ff' stroke-width='0.5' opacity='0.5'><path d='M0 40 H400 M0 80 H400 M0 120 H400 M0 160 H400 M50 0 V200 M120 0 V200 M190 0 V200 M260 0 V200 M330 0 V200'/></g><circle cx='190' cy='100' r='26' fill='none' stroke='%23facc15' stroke-width='1.5'/><path d='M190 66 V134 M156 100 H224' stroke='%23facc15' stroke-width='1'/></svg>";

/// The `/deployments` route: the caller's service record behind the sign-in gate.
#[component]
pub fn DeploymentsPage() -> impl IntoView {
    view! {
        <crate::v2::core::ui::AuthGate>
            <DeploymentsInner />
        </crate::v2::core::ui::AuthGate>
    }
}

/// Whose service record the page shows: the caller's display name and tier.
#[derive(Clone, PartialEq)]
struct RecordHolder {
    username: String,
    role: Role,
}

/// The signed-in half of the page: the one fetch and its two render states.
#[component]
fn DeploymentsInner() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    // Memoized, so the record is rebuilt when the caller's name or tier changes and not on every
    // profile poll; a rebuild discards the leave-of-absence form's unsent input.
    let holder = Memo::new(move |_| {
        store.user.with(|user| {
            user.as_ref().map(|u| RecordHolder {
                username: u.username.clone(),
                role: u.role,
            })
        })
    });
    let data = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Deployments>(store, "/me/deployments")
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
                        Some(d) => dossier(d, holder.get()).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The two-column service record, over a fetched payload and the caller it belongs to.
fn dossier(d: Deployments, holder: Option<RecordHolder>) -> impl IntoView {
    let username = holder
        .as_ref()
        .map(|h| h.username.clone())
        .unwrap_or_default();
    let role = holder.as_ref().map(|h| h.role.as_str()).unwrap_or_default();
    let is_admin = holder
        .as_ref()
        .is_some_and(|h| matches!(h.role, Role::Admin));
    let has_active = !d.upcoming.is_empty();
    let has_history = !d.service_history.is_empty();
    let upcoming = d.upcoming.clone();
    let history = d.service_history.clone();

    view! {
        <div class="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
            <div class="flex h-full w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl lg:flex-row">
                // Left: the identity and deployment-count column.
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

                // Right: active orders, combat history and the leave panels.
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
