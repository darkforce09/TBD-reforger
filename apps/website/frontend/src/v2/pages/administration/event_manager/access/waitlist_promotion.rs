//! The per-mission control that seats waiting participants in the places that are free.
//!
//! **Role:** renders a mission's "Promote from waiting list" button in the access panel, sends the
//! promotion, names the participants it seated, and reads the participant evidence again.
//! **Position:** in each mission's header in the Policies section of the access panel.
//! **Signals & state:** shares the panel's in-flight flag, so a promotion and an access change are
//! never in flight together; writes the panel's notice with what the promotion did.
//! **Invariants:** a promotion is not an access change: it names no revision and leaves the access
//! view as it is. The backend seats the earliest eligible waiters in queue order, so there is no
//! picker. A promotion with no place to give is refused with `EVENT_FULL`, which is worded as such.
//! The request is browser-only; a native build renders the button and sends nothing.

use super::state::AccessPanel;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The report heading and the names of the participants a promotion seated.
pub(super) fn promotion_report(
    mission_title: &str,
    seated: Vec<String>,
) -> super::change_report::ChangeReport {
    super::change_report::ChangeReport {
        change: if seated.is_empty() {
            format!("Nobody waiting on {mission_title} could be seated")
        } else {
            format!("Seated from the waiting list of {mission_title}")
        },
        released: Vec::new(),
        promoted: seated,
    }
}

/// The sentence a refused promotion of `mission_title` is reported with.
pub(super) fn promotion_refusal(
    mission_title: &str,
    code: Option<&str>,
    message: String,
) -> String {
    match code {
        Some("EVENT_FULL") => format!(
            "Nobody waiting on {mission_title} could be seated: the operation or the pools its \
             waiters draw from are full."
        ),
        _ => message,
    }
}

/// The promotion button for one mission.
pub(super) fn waitlist_promotion_button(
    panel: AccessPanel,
    event_mission_id: String,
) -> impl IntoView {
    let mission = StoredValue::new(event_mission_id);
    let promote = move |_| {
        #[cfg(target_arch = "wasm32")]
        send_promotion(panel, mission.get_value());
        #[cfg(not(target_arch = "wasm32"))]
        let _ = mission;
    };
    view! {
        <button
            type="button"
            on:click=promote
            prop:disabled=move || panel.busy.get()
            title="Seat the earliest eligible waiting participants in the places that are free"
            class="flex items-center gap-1.5 rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface transition hover:bg-white/5 disabled:opacity-50"
        >
            <MaterialIcon name="move_up" class="text-sm" />
            "Promote from waiting list"
        </button>
    }
}

/// Send the promotion and report it.
#[cfg(target_arch = "wasm32")]
fn send_promotion(panel: AccessPanel, event_mission_id: String) {
    use super::change_report::PanelNotice;
    use crate::v2::core::api::endpoints::event_registration::promote_waitlisted_participants;
    if panel.busy.get_untracked() {
        return;
    }
    panel.busy.set(true);
    let toasts = crate::v2::core::ui::toast::use_toasts();
    leptos::task::spawn_local(async move {
        let title = panel
            .missions
            .with_untracked(|m| {
                m.loaded().and_then(|missions| {
                    missions
                        .iter()
                        .find(|mission| mission.event_mission_id == event_mission_id)
                        .map(|mission| mission.title.clone())
                })
            })
            .unwrap_or_else(|| "this mission".to_string());
        match promote_waitlisted_participants(panel.store, &event_mission_id).await {
            Ok(answer) => {
                let ids: Vec<String> = answer
                    .promoted
                    .iter()
                    .map(|p| p.registration_id.clone())
                    .collect();
                let before = panel
                    .participants
                    .with_untracked(|p| p.loaded().cloned())
                    .unwrap_or_default();
                let missions = panel
                    .missions
                    .with_untracked(|m| m.loaded().cloned())
                    .unwrap_or_default();
                let seated = super::change_report::describe_registrations(&ids, &before, &missions);
                let report = promotion_report(&title, seated);
                toasts.success(report.change.clone());
                panel.notice.set(Some(PanelNotice::Changed(report)));
                panel.reload_participants();
            }
            Err(refusal) => {
                let sentence = promotion_refusal(
                    &title,
                    refusal.code(),
                    refusal.message_or("Could not promote from the waiting list"),
                );
                toasts.error(sentence.clone());
                panel.notice.set(Some(PanelNotice::Refused(sentence)));
            }
        }
        panel.busy.set(false);
    });
}
