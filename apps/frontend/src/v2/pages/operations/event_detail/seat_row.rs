//! One seat of the selected squad: its role, its occupant or availability, and what may be done
//! with it.
//!
//! **Role:** renders one slot row — number, role, loadout and tag, a lock when the seat's policy
//! restricts the viewer, and on the right the occupant, the manager's assign or clear control, or
//! the seat's availability — plus the inline assign picker a manager opens on an empty seat.
//! **Position:** repeated down the slot list of the squad pane.
//! **Signals & state:** writes the selector's selected-seat and open-picker signals. Reads nothing
//! of its own.
//! **Invariants:** a seat is selectable only when a claim is offered to the viewer, the seat is free
//! and its policy admits them, so a restricted seat can never be put in front of the register
//! button. A restricted seat says which policy restricts it. The manager's controls do not depend
//! on the viewer's own eligibility — a manager seats other people. Every mutation is a
//! browser-only path and does nothing in a native build.
#![allow(dead_code)]

use super::assign_picker::AssignPicker;
use super::registration_access::seat_eligibility::{restriction_reason, seat_admits_viewer};
use crate::v2::core::api::dto::OrbatSlot;
use crate::v2::core::ui::{cn, MaterialIcon, DEFAULT_AVATAR};
use leptos::prelude::*;

/// What the viewer may do in the row's squad, as the squad pane decided it.
#[derive(Clone, Copy)]
pub(super) struct SeatPermissions {
    /// The viewer may claim a seat of this squad: no hold stops them and a claim is offered.
    pub(super) claim_offered: bool,
    /// The viewer holds the squad or administers the operation, and so assigns and clears seats.
    pub(super) can_manage: bool,
    /// Another leader holds the squad.
    pub(super) locked_for_me: bool,
}

/// One seat row, with the assign picker under it while a manager has it open.
pub(super) fn seat_row(
    emid: String,
    slot: OrbatSlot,
    permissions: SeatPermissions,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &changed);
    let SeatPermissions {
        claim_offered,
        can_manage,
        locked_for_me,
    } = permissions;
    let taken = slot.assigned_to.clone().filter(|a| !a.is_empty()).is_some();
    let restricted = !seat_admits_viewer(&slot);
    let reason = restriction_reason(&slot.policy_source);
    let clickable = claim_offered && !taken && !restricted;
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
        cn(&[
            "flex items-center justify-between gap-3 px-4 py-2 text-sm",
            if clickable { "cursor-pointer" } else { "" },
            if selected { "bg-primary/10" } else { "" },
            if clickable && !selected {
                "hover:bg-surface-container"
            } else {
                ""
            },
        ])
    };
    let on_row = move |_| {
        if !clickable {
            return;
        }
        let cur = selected_slot.get_untracked();
        selected_slot.set(if cur.as_deref() == Some(slot_id.as_str()) {
            None
        } else {
            Some(slot_id.clone())
        });
    };
    let status = if taken {
        // Whoever may assign a slot may also clear it, so a manager gets the clear control on a
        // filled row.
        if can_manage {
            let sid_clear = slot_id_assign.clone();
            let emid_clear = emid.clone();
            view! {
                <span class="flex items-center gap-2 text-on-surface-variant">
                    <img src=DEFAULT_AVATAR alt="" class="h-6 w-6 rounded-full" />
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
                                    match crate::v2::core::api::client::api_delete(store, &path)
                                        .await
                                    {
                                        Ok(()) => {
                                            toasts.success("Slot cleared");
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
                    <img src=DEFAULT_AVATAR alt="" class="h-6 w-6 rounded-full" />
                    {assigned_label}
                </span>
            }
            .into_any()
        }
    } else if can_manage {
        let sid = slot_id_assign.clone();
        let sid_label = slot_id_assign.clone();
        view! {
            <button
                type="button"
                on:click=move |ev| {
                    ev.stop_propagation();
                    let cur = assigning.get_untracked();
                    assigning
                        .set(if cur.as_deref() == Some(sid.as_str()) { None } else { Some(sid.clone()) });
                }
                class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-primary"
            >
                {move || {
                    if assigning.get().as_deref() == Some(sid_label.as_str()) {
                        "Cancel"
                    } else {
                        "Assign"
                    }
                }}
            </button>
        }
        .into_any()
    } else if locked_for_me {
        view! { <span class="text-xs text-on-surface-variant">"Reserved"</span> }.into_any()
    } else if restricted {
        view! {
            <span
                class="flex items-center gap-1.5 text-xs text-on-surface-variant"
                title=reason
                data-testid="seat-restricted"
            >
                <MaterialIcon name="lock" class="text-sm" />
                {reason}
            </span>
        }
        .into_any()
    } else if clickable {
        let sid = slot.id.clone();
        let sid_label = slot.id.clone();
        view! {
            <span class=move || {
                let selected = selected_slot.get().as_deref() == Some(sid.as_str());
                cn(&["flex items-center gap-2", if selected { "text-primary" } else { "text-success" }])
            }>
                <span class="h-2 w-2 rounded-full bg-current"></span>
                {move || {
                    if selected_slot.get().as_deref() == Some(sid_label.as_str()) {
                        "Selected"
                    } else {
                        "Available"
                    }
                }}
            </span>
        }
        .into_any()
    } else {
        view! { <span class="text-xs text-on-surface-variant">"Open"</span> }.into_any()
    };

    view! {
        <li>
            <div on:click=on_row class=row_class>
                <span class="flex items-center gap-2">
                    <span class="text-on-surface-variant tabular-nums">{slot.number} ":"</span>
                    <span class="font-medium">{slot.role.clone()}</span>
                    {slot
                        .loadout
                        .clone()
                        .filter(|l| !l.is_empty())
                        .map(|l| view! { <span class="text-on-surface-variant">"(" {l} ")"</span> })}
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
                    {(restricted && taken)
                        .then(|| {
                            view! {
                                <span title=reason aria-label=reason class="text-on-surface-variant">
                                    <MaterialIcon name="lock" class="text-sm" />
                                </span>
                            }
                        })}
                </span>
                <span class="shrink-0">{status}</span>
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
}
