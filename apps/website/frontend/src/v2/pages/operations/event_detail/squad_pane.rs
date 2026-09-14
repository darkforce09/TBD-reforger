//! The selected squad: its reservation header, its slot rows, and the footer copy about it.
//!
//! **Role:** renders the active squad's heading with its reserve or release control, the slot
//! list with each slot's role, loadout tag and occupant or availability marker, and the inline
//! assign picker a manager opens on an empty slot. Owns the reservation rules the footer line
//! and the register button both read.
//! **Position:** the right-hand pane of the slotting selector, and the line in its footer bar.
//! **Signals & state:** writes the selector's selected-slot and open-picker signals, and the
//! reserve and release busy flags. Reads nothing of its own.
//! **Invariants:** a squad reserved by someone else is read-only for everyone but that person
//! and an administrator, and the same rule decides who sees the clear control on a filled slot.
//! Every mutation is a browser-only path and does nothing in a native build.
#![allow(dead_code)]

use super::assign_picker::AssignPicker;
use super::slotting_selector::OrbatBusy;
use crate::v2::core::api::dto::OrbatSquad;
use crate::v2::core::ui::{cn, MaterialIcon, DEFAULT_AVATAR};
use leptos::prelude::*;

/// The three reservation verdicts for a squad, as `(can_manage, locked_for_me, self_register)`.
///
/// `can_manage` is the reserver and any administrator, `locked_for_me` is everyone else while a
/// reservation stands, and `self_register` is the ordinary case where the caller may claim a slot.
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

/// The line in the selector's footer bar: what the caller's current standing on this mission is,
/// or what they are expected to do next.
pub(super) fn footer_message(
    my_state: Option<String>,
    asq: Option<OrbatSquad>,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
) -> String {
    if let Some(s) = my_state {
        return format!("You are {s} for this mission.");
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

/// The active squad's header and slot list.
#[allow(clippy::too_many_arguments)]
pub(super) fn squad_pane(
    emid: String,
    sq: OrbatSquad,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
    my_state: Option<String>,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    busy: OrbatBusy,
    changed: Callback<()>,
    emid_rsv: String,
    emid_rel: String,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let _ = my_state;
    // The store and callback feed only the browser-only mutation closures.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &changed);
    let (can_manage, locked_for_me, self_register) =
        squad_flags(Some(&sq), me.clone(), is_leader, is_admin);
    let reserved_by = sq.reserved_by.clone().filter(|r| !r.is_empty());
    let i_am_reserver = reserved_by.is_some() && reserved_by == me;
    let squad_name = sq.squad.clone();
    let callsign = sq.callsign.clone().filter(|c| !c.is_empty());

    // Reserve this squad for the caller.
    let squad_rsv = squad_name.clone();
    let on_reserve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.reserve.get_untracked() {
                return;
            }
            busy.reserve.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/event-missions/{emid_rsv}/squads/reserve");
            let squad = squad_rsv.clone();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post_ok(
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
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not reserve squad",
                    )),
                }
                busy.reserve.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&emid_rsv, &squad_rsv);
    };

    // Release the reservation.
    let squad_rel = squad_name.clone();
    let on_release = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.release.get_untracked() {
                return;
            }
            busy.release.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/event-missions/{emid_rel}/squads/release");
            let squad = squad_rel.clone();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post_ok(
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
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not release squad",
                    )),
                }
                busy.release.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&emid_rel, &squad_rel);
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
                .iter()
                .map(|slot| {
                    let taken = slot
                        .assigned_to
                        .clone()
                        .filter(|a| !a.is_empty())
                        .is_some();
                    let clickable = self_register && !taken;
                    let slot_id = slot.id.clone();
                    let slot_id_sel = slot.id.clone();
                    let slot_id_assign = slot.id.clone();
                    let slot_id_picker = slot.id.clone();
                    let assigned_label = slot
                        .assigned_name
                        .clone()
                        .filter(|n| !n.is_empty())
                        .or_else(|| slot.assigned_to.clone())
                        .unwrap_or_default();
                    let row_class = move || {
                        let selected = selected_slot.get().as_deref() == Some(slot_id_sel.as_str());
                        cn(
                            &[
                                "flex items-center justify-between gap-3 px-4 py-2 text-sm",
                                if clickable { "cursor-pointer" } else { "" },
                                if selected { "bg-primary/10" } else { "" },
                                if clickable && !selected { "hover:bg-surface-container" } else { "" },
                            ],
                        )
                    };
                    let on_row = move |_| {
                        if !clickable {
                            return;
                        }
                        let cur = selected_slot.get_untracked();
                        selected_slot
                            .set(
                                if cur.as_deref() == Some(slot_id.as_str()) {
                                    None
                                } else {
                                    Some(slot_id.clone())
                                },
                            );
                    };
                    view! {
                        <li>
                            <div on:click=on_row class=row_class>
                                <span class="flex items-center gap-2">
                                    <span class="text-on-surface-variant tabular-nums">
                                        {slot.number}
                                        ":"
                                    </span>
                                    <span class="font-medium">{slot.role.clone()}</span>
                                    {slot
                                        .loadout
                                        .clone()
                                        .filter(|l| !l.is_empty())
                                        .map(|l| {
                                            view! {
                                                <span class="text-on-surface-variant">"(" {l} ")"</span>
                                            }
                                        })}
                                    {slot
                                        .tag
                                        .clone()
                                        .filter(|t| !t.is_empty())
                                        .map(|t| {
                                            view! {
                                                <span class="rounded bg-surface-container-highest px-1.5 py-0.5 text-[10px] font-semibold text-on-surface-variant">
                                                    {t}
                                                </span>
                                            }
                                        })}
                                </span>
                                <span class="shrink-0">
                                    {if taken {
                                        // Whoever may assign a slot may also clear it, so a
                                        // manager gets the clear control on a filled row.
                                        if can_manage {
                                            let sid_clear = slot_id_assign.clone();
                                            let emid_clear = emid.clone();
                                            view! {
                                                <span class="flex items-center gap-2 text-on-surface-variant">
                                                    <img
                                                        src=DEFAULT_AVATAR
                                                        alt=""
                                                        class="h-6 w-6 rounded-full"
                                                    />
                                                    {assigned_label}
                                                    <button
                                                        type="button"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                let toasts = crate::v2::core::ui::toast::use_toasts();
                                                                let path = format!(
                                                                    "/event-missions/{}/slots/{}/assign",
                                                                    emid_clear,
                                                                    sid_clear
                                                                );
                                                                leptos::task::spawn_local(async move {
                                                                    match crate::v2::core::api::client::api_delete(
                                                                        store, &path,
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(()) => {
                                                                            toasts.success(
                                                                                "Slot cleared",
                                                                            );
                                                                            changed.run(());
                                                                        }
                                                                        Err(e) => toasts.error(
                                                                            crate::v2::core::api::client::api_error_message(
                                                                                &e,
                                                                                "Could not clear slot",
                                                                            ),
                                                                        ),
                                                                    }
                                                                });
                                                            }
                                                            #[cfg(not(target_arch = "wasm32"))]
                                                            let _ = (&emid_clear, &sid_clear);
                                                        }
                                                        class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-error"
                                                    >
                                                        "Clear"
                                                    </button>
                                                </span>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <span class="flex items-center gap-2 text-on-surface-variant">
                                                    <img
                                                        src=DEFAULT_AVATAR
                                                        alt=""
                                                        class="h-6 w-6 rounded-full"
                                                    />
                                                    {assigned_label}
                                                </span>
                                            }
                                                .into_any()
                                        }
                                    } else if can_manage {
                                        let sid = slot_id_assign.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let cur = assigning.get_untracked();
                                                    assigning
                                                        .set(
                                                            if cur.as_deref() == Some(sid.as_str()) {
                                                                None
                                                            } else {
                                                                Some(sid.clone())
                                                            },
                                                        );
                                                }
                                                class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-primary"
                                            >
                                                {
                                                    let sid = slot_id_assign.clone();
                                                    move || {
                                                        if assigning.get().as_deref() == Some(sid.as_str()) {
                                                            "Cancel"
                                                        } else {
                                                            "Assign"
                                                        }
                                                    }
                                                }
                                            </button>
                                        }
                                            .into_any()
                                    } else if locked_for_me {
                                        view! {
                                            <span class="text-xs text-on-surface-variant">"Reserved"</span>
                                        }
                                            .into_any()
                                    } else {
                                        let sid = slot.id.clone();
                                        view! {
                                            <span class=move || {
                                                let selected = selected_slot.get().as_deref()
                                                    == Some(sid.as_str());
                                                cn(
                                                    &[
                                                        "flex items-center gap-2",
                                                        if selected { "text-primary" } else { "text-success" },
                                                    ],
                                                )
                                            }>
                                                <span class="h-2 w-2 rounded-full bg-current"></span>
                                                {
                                                    let sid = slot.id.clone();
                                                    move || {
                                                        if selected_slot.get().as_deref() == Some(sid.as_str()) {
                                                            "Selected"
                                                        } else {
                                                            "Available"
                                                        }
                                                    }
                                                }
                                            </span>
                                        }
                                            .into_any()
                                    }}
                                </span>
                            </div>
                            {(can_manage && !taken)
                                .then(|| {
                                    let sid = slot_id_picker.clone();
                                    let emid = emid.clone();
                                    move || {
                                        (assigning.get().as_deref() == Some(sid.as_str()))
                                            .then(|| {
                                                view! {
                                                    <AssignPicker
                                                        emid=emid.clone()
                                                        slot_id=sid.clone()
                                                        assigning=assigning
                                                        changed=changed
                                                    />
                                                }
                                            })
                                    }
                                })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
