//! The inline slotting selector: faction tabs, the squad list, and the actions on them.
//!
//! **Role:** fetches a mission's order of battle, resolves which faction and squad are active,
//! and renders the two-column selector — the faction tabs and squad list on the left, the
//! selected squad's slots and the footer action bar on the right — with the viewer's standing on
//! the mission deciding which seats and which registration actions are offered.
//! **Position:** the bottom of every mission dossier card, and the whole body of the standalone
//! slotting route.
//! **Signals & state:** owns the order-of-battle resource and the four selection signals
//! (faction, squad, selected slot, the slot whose assign picker is open), the per-mutation busy
//! flags, and the refusal notice of the last failed registration. Reads the session store for the
//! caller's identity and tier, as memos so a tier that arrives after the first paint still reaches
//! the affordances.
//! **Invariants:** the tier checks go through the authenticated form, so a browse-mode session
//! with no user is never treated as a leader or an administrator. A partial viewer's order of
//! battle carries only the seats open to them, and the selector shows nothing more. Every mutation
//! is a browser-only path; a native build renders the selector and does nothing on click.
#![allow(dead_code)]

use super::faction_armory::sort_factions;
use super::registration_access::mission_standing::MissionStanding;
use super::registration_access::refusal_notices::RegistrationRefusal;
use super::reservation_actions::{reservation_footer, FooterViewer};
use super::squad_pane::squad_pane;
use crate::v2::core::api::dto::{DataEnvelope, OrbatSquad};
use crate::v2::core::auth::{has_min_role_authed, Role};
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// Only an active reservation or waiting-list entry can be withdrawn.
pub(super) fn can_withdraw_reservation(state: Option<&str>) -> bool {
    matches!(state, Some("registered" | "waitlisted"))
}

/// Retained history permits another registration; unknown states offer no mutation.
pub(super) fn can_register_reservation(state: Option<&str>) -> bool {
    matches!(state, None | Some("withdrawn" | "legacy_unknown"))
}

/// One busy flag per mutation, so a double click cannot post twice.
#[derive(Clone, Copy)]
pub(super) struct OrbatBusy {
    /// A register request is in flight.
    pub(super) register: RwSignal<bool>,
    /// A withdraw request is in flight.
    pub(super) withdraw: RwSignal<bool>,
    /// A squad reservation request is in flight.
    pub(super) reserve: RwSignal<bool>,
    /// A squad release request is in flight.
    pub(super) release: RwSignal<bool>,
}

/// The inline order-of-battle selector: faction tabs, squad list, slot rows, and the register,
/// join-the-waiting-list, withdraw, reserve, release and assign actions on them.
///
/// `standing` is the viewer's standing on this mission; its reservation state controls allocation
/// actions independently of attendance history. `on_change` reloads the caller's mission data after
/// every successful mutation.
#[component]
pub fn OrbatSelector(
    emid: String,
    standing: MissionStanding,
    #[prop(optional)] on_change: Option<Callback<()>>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let emid_res = emid.clone();
    let orbat = LocalResource::new(move || {
        let emid = emid_res.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/event-missions/{emid}/orbat");
                crate::v2::core::api::client::api_get::<DataEnvelope<OrbatSquad>>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, emid);
                None::<DataEnvelope<OrbatSquad>>
            }
        }
    });
    // The faction and squad selections hold an option resolved against the live list, so a
    // squad that disappears between fetches falls back to the first one rather than to nothing.
    let faction_sel = RwSignal::new(None::<String>);
    let squad_sel = RwSignal::new(None::<String>);
    let selected_slot = RwSignal::new(None::<String>);
    let assigning = RwSignal::new(None::<String>);
    let busy = OrbatBusy {
        register: RwSignal::new(false),
        withdraw: RwSignal::new(false),
        reserve: RwSignal::new(false),
        release: RwSignal::new(false),
    };
    let refusal = RwSignal::new(None::<RegistrationRefusal>);
    // A mutation reloads the order of battle here, then bubbles to whatever mounted it.
    let changed = Callback::new(move |()| {
        orbat.refetch();
        if let Some(cb) = on_change {
            cb.run(());
        }
    });

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-sm text-on-surface-variant">"Loading ORBAT…"</p> }
        }>
            {move || {
                let emid = emid.clone();
                let standing = standing.clone();
                orbat
                    .get()
                    .map(move |opt| {
                        let squads = opt.map(|e| e.data).unwrap_or_default();
                        if squads.is_empty() {
                            view! {
                                <p class="text-sm text-on-surface-variant">
                                    "No ORBAT slots defined for this mission."
                                </p>
                            }
                                .into_any()
                        } else {
                            selector_shell(
                                    emid.clone(),
                                    standing.clone(),
                                    squads,
                                    faction_sel,
                                    squad_sel,
                                    selected_slot,
                                    assigning,
                                    busy,
                                    refusal,
                                    changed,
                                )
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The selector body, for a mission that has slots.
///
/// Split out of the component so the suspense closure stays readable.
#[allow(clippy::too_many_arguments)]
pub(super) fn selector_shell(
    emid: String,
    standing: MissionStanding,
    squads: Vec<OrbatSquad>,
    faction_sel: RwSignal<Option<String>>,
    squad_sel: RwSignal<Option<String>>,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    busy: OrbatBusy,
    refusal: RwSignal<Option<RegistrationRefusal>>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let factions = sort_factions(
        squads
            .iter()
            .map(|s| s.faction.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
    );
    // Reactive and authenticated: a browse-mode session answers "yes" to an unauthenticated
    // tier check, and that must never drive the reserve, release or assign affordances.
    let is_leader =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Leader));
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    // Non-`Copy` captures ride stored values so every closure below stays `Copy`; they are used
    // repeatedly inside reactive renders.
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));
    let seat_claim_offered = standing.seat_claim_offered();

    let factions_for_tabs = factions.clone();
    let squads_sv = StoredValue::new(squads);
    let factions_sv = StoredValue::new(factions);

    // The resolved active faction and squad: the pick when it still exists, else the first.
    let active = move || {
        let factions = factions_sv.get_value();
        let af = faction_sel
            .get()
            .filter(|f| factions.contains(f))
            .or_else(|| factions.first().cloned());
        let fsquads: Vec<OrbatSquad> = squads_sv
            .get_value()
            .into_iter()
            .filter(|s| Some(&s.faction) == af.as_ref())
            .collect();
        let asq = squad_sel
            .get()
            .and_then(|k| fsquads.iter().find(|s| s.squad == k).cloned())
            .or_else(|| fsquads.first().cloned());
        (af, fsquads, asq)
    };

    let pick_squad = move |squad: String| {
        squad_sel.set(Some(squad));
        selected_slot.set(None);
        assigning.set(None);
    };

    let emid_rsv = emid.clone();
    let emid_rel = emid.clone();
    let emid_assign = emid.clone();
    let active_squad = move || active().2;
    let viewer = FooterViewer {
        me,
        is_leader,
        is_admin,
    };

    view! {
        <div class="grid overflow-hidden rounded-xl border border-border-subtle md:grid-cols-[240px_1fr]">
            // Left: navigation sidebar
            <aside class="border-b border-border-subtle bg-surface-container p-4 md:border-b-0 md:border-r">
                {(factions_for_tabs.len() > 1)
                    .then(|| {
                        let tabs = factions_for_tabs.clone();
                        view! {
                            <div class="mb-4 flex rounded-lg bg-surface p-1">
                                {tabs
                                    .into_iter()
                                    .map(|f| {
                                        let f_click = f.clone();
                                        let f_active = f.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    faction_sel.set(Some(f_click.clone()));
                                                    squad_sel.set(None);
                                                    selected_slot.set(None);
                                                    assigning.set(None);
                                                }
                                                class=move || {
                                                    let (af, _, _) = active();
                                                    cn(
                                                        &[
                                                            "flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                                                            if af.as_deref() == Some(f_active.as_str()) {
                                                                "bg-primary text-on-primary"
                                                            } else {
                                                                "text-on-surface-variant"
                                                            },
                                                        ],
                                                    )
                                                }
                                            >
                                                {f.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })}
                <ul class="space-y-1">
                    {move || {
                        let (_, fsquads, asq) = active();
                        fsquads
                            .into_iter()
                            .map(|s| {
                                let is_active = asq.as_ref().map(|a| a.squad == s.squad).unwrap_or(false);
                                let squad_name = s.squad.clone();
                                let full = s.filled >= s.total;
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            on:click=move |_| pick_squad(squad_name.clone())
                                            class=cn(
                                                &[
                                                    "flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition-colors",
                                                    if is_active {
                                                        "bg-primary/10 text-on-surface"
                                                    } else {
                                                        "text-on-surface-variant hover:bg-surface-container-high"
                                                    },
                                                ],
                                            )
                                        >
                                            <span class="flex items-center gap-1.5">
                                                {s.reserved_by
                                                    .is_some()
                                                    .then(|| {
                                                        view! {
                                                            <MaterialIcon
                                                                name="lock"
                                                                class="text-sm text-on-surface-variant"
                                                            />
                                                        }
                                                    })}
                                                <span class="font-medium text-on-surface">
                                                    {s.squad.clone()}
                                                </span>
                                                {s.callsign
                                                    .clone()
                                                    .filter(|c| !c.is_empty())
                                                    .map(|c| view! { <span class="ml-1 text-xs">{c}</span> })}
                                            </span>
                                            <span class=cn(
                                                &[
                                                    "font-mono text-xs",
                                                    if full { "text-error" } else { "text-on-surface-variant" },
                                                ],
                                            )>{s.filled} "/" {s.total}</span>
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </aside>

            // Right: slot detail pane
            <section class="flex min-h-[18rem] flex-col bg-surface-container-high">
                <div class="flex-1 p-4">
                    {move || {
                        let (_, _, asq) = active();
                        match asq {
                            Some(sq) => {
                                squad_pane(
                                        emid_assign.clone(),
                                        sq,
                                        me.get_value(),
                                        is_leader.get(),
                                        is_admin.get(),
                                        seat_claim_offered,
                                        selected_slot,
                                        assigning,
                                        busy,
                                        changed,
                                        emid_rsv.clone(),
                                        emid_rel.clone(),
                                    )
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <p class="text-on-surface-variant">
                                        "Select a squad to view its slots."
                                    </p>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </div>

                {reservation_footer(
                    emid,
                    standing,
                    active_squad,
                    viewer,
                    selected_slot,
                    busy,
                    refusal,
                    changed,
                )}
            </section>
        </div>
    }
}
