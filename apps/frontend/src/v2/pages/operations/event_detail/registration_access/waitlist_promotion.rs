//! The leader's control that moves a mission's waiting participants into free places.
//!
//! **Role:** renders the "Promote from waiting list" button on a mission card for a leader or an
//! administrator, sends the promotion, and reports how many waiting participants were seated or
//! why none could be.
//! **Position:** in the header column of each mission card, under the fill counts.
//! **Signals & state:** owns a busy flag; reads the session store for the viewer's tier, as a memo,
//! so a tier that arrives after the first paint still reaches the control. Runs the card's refetch
//! callback after a promotion.
//! **Invariants:** the tier check goes through the authenticated form, so a browse-mode session is
//! never treated as a leader. The request cannot choose who is promoted — the backend seats the
//! earliest eligible waiters in queue order — so the control has no picker. A promotion with no
//! place to give is refused with `EVENT_FULL`, which is worded as such rather than as a failure.
//! The request is browser-only; a native build renders the control and sends nothing.

use crate::v2::core::auth::{has_min_role_authed, Role};
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The sentence a promotion answer is reported with.
pub(crate) fn promotion_summary(promoted: usize) -> String {
    match promoted {
        0 => "Nobody on the waiting list could be seated.".to_string(),
        1 => "Seated 1 participant from the waiting list.".to_string(),
        n => format!("Seated {n} participants from the waiting list."),
    }
}

/// The sentence a refused promotion is reported with.
pub(crate) fn promotion_refusal_sentence(code: Option<&str>, message: String) -> String {
    match code {
        Some("EVENT_FULL") => {
            "No place is free for anyone waiting: the operation or its pools are full.".to_string()
        }
        _ => message,
    }
}

/// The promotion control for one mission, shown to leaders and administrators only.
pub(crate) fn waitlist_promotion_control(
    event_mission_id: String,
    on_change: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let is_leader =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Leader));
    let busy = RwSignal::new(false);
    let mission = StoredValue::new(event_mission_id);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&on_change, mission);
    let promote = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let emid = mission.get_value();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::endpoints::event_registration::promote_waitlisted_participants(
                    store, &emid,
                )
                .await
                {
                    Ok(answer) => {
                        toasts.success(promotion_summary(answer.promoted.len()));
                        on_change.run(());
                    }
                    Err(refusal) => toasts.error(promotion_refusal_sentence(
                        refusal.code(),
                        refusal.message_or("Could not promote from the waiting list"),
                    )),
                }
                busy.set(false);
            });
        }
    };
    move || {
        is_leader.get().then(|| {
            view! {
                <button
                    type="button"
                    on:click=promote
                    prop:disabled=move || busy.get()
                    data-testid="waitlist-promote"
                    title="Seat the earliest eligible waiting participants in free places"
                    class="flex items-center gap-2 rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-on-surface transition hover:bg-white/5 disabled:opacity-50"
                >
                    <MaterialIcon name="move_up" class="text-base" />
                    " Promote from waiting list"
                </button>
            }
        })
    }
}
