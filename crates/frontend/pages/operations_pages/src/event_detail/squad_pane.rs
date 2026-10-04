//! The selected squad: its reservation header, its slot rows, and the footer copy about it.
//!
//! **Role:** renders the active squad's heading with its reserve or release control and the slot
//! list, one seat row per slot, and owns the squad-hold rules the seat rows, the footer line and
//! the register button all read.
//! **Position:** the right-hand pane of the slotting selector, and the line in its footer bar.
//! **Signals & state:** hands the selector's selected-slot and open-picker signals to the seat
//! rows, and writes the reserve and release busy flags. Reads nothing of its own.
//! **Invariants:** a squad reserved by someone else is read-only for everyone but that person and
//! an administrator, and the same rule decides who sees the clear control on a filled slot. A seat
//! is claimable only when the squad allows the viewer to claim and a claim is offered on the
//! mission at all. Every mutation is a browser-only path, as is the view that offers it.

#[cfg(target_arch = "wasm32")]
use super::seat_row::{SeatPermissions, seat_row};
#[cfg(target_arch = "wasm32")]
use super::slotting_selector::OrbatBusy;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::OrbatSquad;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The three reservation verdicts for a squad, as `(can_manage, locked_for_me, self_register)`.
///
/// `can_manage` is the reserver and any administrator, `locked_for_me` is everyone else while a
/// reservation stands, and `self_register` is the ordinary case where the caller may claim a slot.
#[cfg(target_arch = "wasm32")]
pub(super) fn squad_flags(
    sq: Option<&OrbatSquad>,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
) -> (bool, bool, bool) {
    let _ = is_leader;
    let Some(sq) = sq else {
        return (false, false, false);
    };
    let reserved_by = sq.reserved_by.clone().filter(|r| !r.is_empty());
    let i_am_reserver = reserved_by.is_some() && reserved_by == me;
    let can_manage = is_admin || i_am_reserver;
    let locked_for_me = reserved_by.is_some() && !can_manage;
    let self_register = !can_manage && !locked_for_me;
    (can_manage, locked_for_me, self_register)
}

/// The line in the selector's footer bar: the caller's own signup on this mission when they have
/// one, or what they are expected to do next.
#[cfg(target_arch = "wasm32")]
pub(super) fn footer_message(
    signup_line: Option<String>,
    asq: Option<OrbatSquad>,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
) -> String {
    if let Some(line) = signup_line {
        return line;
    }
    let (can_manage, locked_for_me, _) = squad_flags(asq.as_ref(), me, is_leader, is_admin);
    if locked_for_me {
        "This squad is reserved by a leader.".to_string()
    } else if can_manage {
        "Assign members to fill this squad.".to_string()
    } else {
        "Select an open slot to deploy.".to_string()
    }
}

// Separate parameters: the squad, the viewer's flags and the selector's signals, one caller.
/// The active squad's header and slot list.
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub(super) fn squad_pane(
    emid: String,
    sq: OrbatSquad,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
    seat_claim_offered: bool,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    busy: OrbatBusy,
    changed: Callback<()>,
    emid_rsv: String,
    emid_rel: String,
) -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    let (can_manage, locked_for_me, self_register) =
        squad_flags(Some(&sq), me.clone(), is_leader, is_admin);
    let permissions = SeatPermissions {
        claim_offered: self_register && seat_claim_offered,
        can_manage,
        locked_for_me,
    };
    let reserved_by = sq.reserved_by.clone().filter(|r| !r.is_empty());
    let i_am_reserver = reserved_by.is_some() && reserved_by == me;
    let squad_name = sq.squad.clone();
    let callsign = sq.callsign.clone().filter(|c| !c.is_empty());

    // Reserve this squad for the caller.
    let squad_rsv = squad_name.clone();
    let on_reserve = move |_| {
        if busy.reserve.get_untracked() {
            return;
        }
        busy.reserve.set(true);
        let toasts = frontend_ui::toast::use_toasts();
        let path = format!("/event-missions/{emid_rsv}/squads/reserve");
        let squad = squad_rsv.clone();
        leptos::task::spawn_local(async move {
            match frontend_transport::client::api_post_ok(
                store,
                &path,
                serde_json::json!({ "squad": squad }),
            )
            .await
            {
                Ok(()) => {
                    toasts.success(format!("Reserved {squad}"));
                    changed.run(());
                }
                Err(e) => toasts.error(e.message_or("Could not reserve squad")),
            }
            busy.reserve.set(false);
        });
    };

    // Release the reservation.
    let squad_rel = squad_name.clone();
    let on_release = move |_| {
        if busy.release.get_untracked() {
            return;
        }
        busy.release.set(true);
        let toasts = frontend_ui::toast::use_toasts();
        let path = format!("/event-missions/{emid_rel}/squads/release");
        let squad = squad_rel.clone();
        leptos::task::spawn_local(async move {
            match frontend_transport::client::api_post_ok(
                store,
                &path,
                serde_json::json!({ "squad": squad }),
            )
            .await
            {
                Ok(()) => {
                    toasts.success("Squad released");
                    changed.run(());
                }
                Err(e) => toasts.error(e.message_or("Could not release squad")),
            }
            busy.release.set(false);
        });
    };

    view! {
        <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
            <h4 class="font-semibold">
                {squad_name.clone()}
                {callsign
                    .map(|c| {
                        view! {
                            <span class="text-sm font-normal text-on-surface-variant">
                                " | "
                                {c}
                            </span>
                        }
                    })}
            </h4>
            <div class="flex items-center gap-2">
                {if reserved_by.is_some() {
                    let holder = sq
                        .reserved_by_name
                        .clone()
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "a leader".into());
                    view! {
                        <span class="flex items-center gap-1 rounded bg-surface-container-highest px-2 py-0.5 text-xs text-on-surface-variant">
                            <MaterialIcon name="lock" class="text-sm" />
                            "Reserved by "
                            {holder}
                        </span>
                        {(i_am_reserver || is_admin)
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=on_release
                                        prop:disabled=move || busy.release.get()
                                        class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-on-surface-variant disabled:opacity-50"
                                    >
                                        "Release"
                                    </button>
                                }
                            })}
                    }
                        .into_any()
                } else if is_leader {
                    view! {
                        <button
                            type="button"
                            on:click=on_reserve
                            prop:disabled=move || busy.reserve.get()
                            class="flex items-center gap-1 rounded-lg bg-primary px-3 py-1 text-xs font-medium text-on-primary disabled:opacity-50"
                        >
                            <MaterialIcon name="lock" class="text-sm" />
                            " Reserve Squad"
                        </button>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </div>
        </div>

        <ul class="overflow-hidden rounded-lg border border-border-subtle divide-y divide-border-subtle">
            {sq
                .slots
                .into_iter()
                .map(|slot| {
                    seat_row(emid.clone(), slot, permissions, selected_slot, assigning, changed)
                })
                .collect_view()}
        </ul>
    }
}
