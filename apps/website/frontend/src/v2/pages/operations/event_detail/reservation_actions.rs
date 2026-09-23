//! The slotting footer: the viewer's standing, the registration actions, and why one was refused.
//!
//! **Role:** renders the selector's footer bar — the line about the viewer's own signup or what to
//! do next, the refusal notice of the last failed request, and the withdraw, join-the-waiting-list
//! and register buttons — and sends those three requests.
//! **Position:** the bottom bar of the slotting selector's right-hand pane.
//! **Signals & state:** reads the selected seat, and writes it clear after a registration; sets the
//! register and withdraw busy flags around their requests; writes the refusal notice, which the
//! selector owns so that it outlives an order-of-battle refetch, and clears it when a request
//! succeeds. Runs the selector's change callback after every success.
//! **Invariants:** the buttons offered follow the viewer's standing — withdraw for an active or
//! waiting signup, register only while a place can be had and a claimable seat is picked, and join
//! the waiting list only when a seat of the mission admits the viewer and there is no place or seat
//! to take, or a refusal has just said so. Joining the waiting list is a registration without a
//! seat, which the backend waitlists when no place is free. A refusal is worded by its reason
//! rather than shown as a generic failure. Every request is browser-only.
#![allow(dead_code)]

use super::registration_access::mission_standing::MissionStanding;
use super::registration_access::refusal_notices::RegistrationRefusal;
use super::slotting_selector::{can_withdraw_reservation, OrbatBusy};
use super::squad_pane::{footer_message, squad_flags};
use crate::v2::core::api::dto::OrbatSquad;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// Who is looking at the footer, as the selector resolved it.
#[derive(Clone, Copy)]
pub(super) struct FooterViewer {
    /// The viewer's Discord id, when signed in.
    pub(super) me: StoredValue<Option<String>>,
    pub(super) is_leader: Memo<bool>,
    pub(super) is_admin: Memo<bool>,
}

/// The sentence a successful registration answer is reported with.
pub(super) fn registration_success(reservation_state: &str, seat_asked: bool) -> &'static str {
    match (reservation_state, seat_asked) {
        ("waitlisted", _) => "Added to the waiting list",
        (_, true) => "Registered for deployment",
        (_, false) => "Registered: a place is held for you without a seat",
    }
}

/// The footer bar of the slotting selector.
#[allow(clippy::too_many_arguments)]
pub(super) fn reservation_footer(
    emid: String,
    standing: MissionStanding,
    active_squad: impl Fn() -> Option<OrbatSquad> + Copy + Send + Sync + 'static,
    viewer: FooterViewer,
    selected_slot: RwSignal<Option<String>>,
    busy: OrbatBusy,
    refusal: RwSignal<Option<RegistrationRefusal>>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let standing = StoredValue::new(standing);
    let emid = StoredValue::new(emid);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, emid, &changed);

    // Register for the selected seat, or — with `seat` empty — for a seatless place, which is how
    // the waiting list is joined. One busy flag covers both: they are the same route.
    let send_registration = move |seat: Option<String>| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.register.get_untracked() {
                return;
            }
            busy.register.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let mission = emid.get_value();
            leptos::task::spawn_local(async move {
                let asked = seat.is_some();
                match crate::v2::core::api::endpoints::event_registration::register_for_mission(
                    store,
                    &mission,
                    seat.as_deref(),
                )
                .await
                {
                    Ok(answer) => {
                        refusal.set(None);
                        toasts.success(registration_success(&answer.reservation_state, asked));
                        selected_slot.set(None);
                        changed.run(());
                    }
                    Err(failure) => refusal.set(Some(RegistrationRefusal::from_refusal(
                        &failure,
                        if asked {
                            "Could not claim that slot"
                        } else {
                            "Could not join the waiting list"
                        },
                    ))),
                }
                busy.register.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = seat;
    };
    let on_register = move |_| {
        if let Some(seat) = selected_slot.get_untracked() {
            send_registration(Some(seat));
        }
    };
    let on_join_waiting_list = move |_| send_registration(None);

    let on_withdraw = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.withdraw.get_untracked() {
                return;
            }
            busy.withdraw.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path =
                crate::v2::core::api::endpoints::event_registration::mission_registration_path(
                    &emid.get_value(),
                );
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(store, &path).await {
                    Ok(()) => {
                        refusal.set(None);
                        toasts.success("Withdrawn from mission");
                        changed.run(());
                    }
                    Err(_) => toasts.error("Could not withdraw"),
                }
                busy.withdraw.set(false);
            });
        }
    };

    let waiting = standing.with_value(|s| s.reservation_state.as_deref() == Some("waitlisted"));
    let withdraw_offered =
        standing.with_value(|s| can_withdraw_reservation(s.reservation_state.as_deref()));

    view! {
        <div class="border-t border-border-subtle bg-surface-container p-4">
            {move || {
                refusal
                    .get()
                    .map(|notice| {
                        let sentence =
                            notice.sentence(crate::v2::core::utils::datefmt::format_local_datetime);
                        view! {
                            <p
                                class="mb-3 flex items-start gap-2 rounded-lg border border-error-alert/30 bg-error-alert/10 px-3 py-2 text-sm text-error-alert"
                                role="alert"
                                data-testid="registration-refusal"
                            >
                                <MaterialIcon name="block" class="text-base" />
                                <span>{sentence}</span>
                            </p>
                        }
                    })
            }}
            <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="text-sm text-on-surface-variant">
                    {move || {
                        footer_message(
                            standing.with_value(MissionStanding::signup_line),
                            active_squad(),
                            viewer.me.get_value(),
                            viewer.is_leader.get(),
                            viewer.is_admin.get(),
                        )
                    }}
                </div>
                <div class="flex flex-wrap gap-2">
                    {withdraw_offered
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    on:click=on_withdraw
                                    prop:disabled=move || busy.withdraw.get()
                                    class="rounded-lg border border-error/50 px-4 py-2 text-sm text-error disabled:opacity-50"
                                >
                                    {if waiting { "Leave waiting list" } else { "Withdraw" }}
                                </button>
                            }
                        })}
                    {move || {
                        let suggested = refusal
                            .get()
                            .is_some_and(|notice| notice.suggests_waiting_list());
                        standing
                            .with_value(|s| s.waiting_list_offered(suggested))
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=on_join_waiting_list
                                        prop:disabled=move || busy.register.get()
                                        data-testid="join-waiting-list"
                                        class="rounded-lg border border-primary/60 px-4 py-2 text-sm text-primary disabled:opacity-50"
                                    >
                                        "Join waiting list"
                                    </button>
                                }
                            })
                    }}
                    {move || {
                        let (_, _, self_register) = squad_flags(
                            active_squad().as_ref(),
                            viewer.me.get_value(),
                            viewer.is_leader.get(),
                            viewer.is_admin.get(),
                        );
                        (standing.with_value(MissionStanding::seat_claim_offered) && self_register)
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=on_register
                                        prop:disabled=move || {
                                            selected_slot.get().is_none() || busy.register.get()
                                        }
                                        class="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                                    >
                                        "Register for Deployment"
                                    </button>
                                }
                            })
                    }}
                </div>
            </div>
        </div>
    }
}
