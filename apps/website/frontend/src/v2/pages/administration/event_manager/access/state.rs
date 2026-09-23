//! The access panel's state: which operation it is open on, what it has read, and the one path
//! every change goes through.
//!
//! **Role:** one copyable handle holding the operation the panel is open on, its three reads — the
//! access view, the participant evidence, and the missions with their orders of battle — the
//! in-flight flag, the section on screen and the notice about the last change; and the methods that
//! read, send a change, adopt its answer and handle its refusal.
//! **Position:** created by the operations calendar's state; the calendar's day panel opens it and
//! every section of the panel reads and writes through it.
//! **Signals & state:** every field but the store is a signal. A read writes its signal only while
//! the panel is still open on the operation it was asked for, so an answer that arrives after the
//! operator moved to another operation is dropped rather than shown against the wrong one.
//! **Invariants:** every change is sent with the access revision the panel last read and answers
//! the access view after it, which replaces the panel's view without a second read. A stale
//! revision reloads the view and tells the operator the change was not applied. The participant
//! evidence is read again after every change, because a change can release, promote or re-admit
//! anyone. The reads and the change path are browser-only; a native build keeps every signal idle.

use super::change_report::PanelNotice;
use crate::v2::core::api::dto::{
    EventAccessAdministration, OrbatSquad, ParticipantAccessExplanation,
};
use crate::v2::core::auth::AuthStore;
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::change_report::{
    change_refusal_sentence, describe_registrations, is_revision_conflict, ChangeReport,
};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::client::ApiRefusal;
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::AccessChangeOutcome;

/// One read of the panel: not asked for, in flight, refused with a reason, or answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Loadable<T> {
    Idle,
    Loading,
    Failed(String),
    Loaded(T),
}

impl<T> Loadable<T> {
    /// The answer, when there is one.
    pub(super) fn loaded(&self) -> Option<&T> {
        match self {
            Loadable::Loaded(value) => Some(value),
            _ => None,
        }
    }
}

/// One mission of the operation with its order of battle, for the squad and slot policy lists.
#[derive(Clone, PartialEq)]
pub(super) struct MissionSeats {
    pub(super) event_mission_id: String,
    pub(super) title: String,
    pub(super) squads: Vec<OrbatSquad>,
}

/// The panel's four sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AccessTab {
    Policies,
    Groups,
    Places,
    Participants,
}

/// Every signal the access panel runs on.
#[derive(Clone, Copy)]
pub(in super::super) struct AccessPanel {
    /// The session, for every request the panel makes.
    pub(super) store: AuthStore,
    /// Whether the panel is open.
    pub(super) open: RwSignal<bool>,
    /// The operation the panel is open on.
    pub(super) event_id: RwSignal<Option<String>>,
    /// The operation's display name, for the heading.
    pub(super) event_name: RwSignal<String>,
    pub(super) access: RwSignal<Loadable<EventAccessAdministration>>,
    pub(super) participants: RwSignal<Loadable<Vec<ParticipantAccessExplanation>>>,
    pub(super) missions: RwSignal<Loadable<Vec<MissionSeats>>>,
    /// Set while a change is in flight, so a second click cannot send a second change.
    pub(super) busy: RwSignal<bool>,
    /// What the last change did, or why it was refused.
    pub(super) notice: RwSignal<Option<PanelNotice>>,
    /// The section on screen.
    pub(super) tab: RwSignal<AccessTab>,
}

impl AccessPanel {
    /// A shut panel. Called from the calendar's state, inside the route component, so the signals
    /// belong to the route and are disposed with it.
    pub(in super::super) fn new(store: AuthStore) -> Self {
        Self {
            store,
            open: RwSignal::new(false),
            event_id: RwSignal::new(None),
            event_name: RwSignal::new(String::new()),
            access: RwSignal::new(Loadable::Idle),
            participants: RwSignal::new(Loadable::Idle),
            missions: RwSignal::new(Loadable::Idle),
            busy: RwSignal::new(false),
            notice: RwSignal::new(None),
            tab: RwSignal::new(AccessTab::Policies),
        }
    }

    /// Open the panel on one operation and read everything it shows.
    pub(in super::super) fn open_on(self, event_id: String, event_name: String) {
        self.event_id.set(Some(event_id));
        self.event_name.set(event_name);
        self.notice.set(None);
        self.tab.set(AccessTab::Policies);
        self.open.set(true);
        self.reload_access();
        self.reload_participants();
        self.reload_missions();
    }

    /// The revision the next change must name, while the view is loaded.
    pub(super) fn revision(self) -> Option<i64> {
        self.access
            .with_untracked(|access| access.loaded().map(|view| view.access_revision))
    }

    /// Read the access view again.
    pub(super) fn reload_access(self) {
        let Some(event) = self.event_id.get_untracked() else {
            return;
        };
        self.access.set(Loadable::Loading);
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            use crate::v2::core::api::endpoints::event_access_administration::load_event_access;
            let read = load_event_access(self.store, &event).await;
            if self.event_id.get_untracked().as_deref() == Some(event.as_str()) {
                self.access.set(match read {
                    Ok(view) => Loadable::Loaded(view),
                    Err(e) => Loadable::Failed(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not load the access settings",
                    )),
                });
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = event;
    }

    /// Read the participant evidence again.
    pub(super) fn reload_participants(self) {
        let Some(event) = self.event_id.get_untracked() else {
            return;
        };
        self.participants.set(Loadable::Loading);
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            use crate::v2::core::api::endpoints::event_access_administration::load_access_participants;
            let read = load_access_participants(self.store, &event).await;
            if self.event_id.get_untracked().as_deref() == Some(event.as_str()) {
                self.participants.set(match read {
                    Ok(list) => Loadable::Loaded(list),
                    Err(e) => Loadable::Failed(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not load the participants",
                    )),
                });
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = event;
    }

    /// Read the operation's missions and each one's order of battle again.
    pub(super) fn reload_missions(self) {
        let Some(event) = self.event_id.get_untracked() else {
            return;
        };
        self.missions.set(Loadable::Loading);
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            let read = read_missions(self.store, &event).await;
            if self.event_id.get_untracked().as_deref() == Some(event.as_str()) {
                self.missions.set(match read {
                    Ok(missions) => Loadable::Loaded(missions),
                    Err(e) => Loadable::Failed(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not load the missions",
                    )),
                });
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = event;
    }

    /// Send one change: `send` receives the session, the operation and the revision to name, and
    /// answers the access view after the change. `change` names it in the report and the toast.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn apply<F>(self, change: String, send: F)
    where
        F: FnOnce(
                AuthStore,
                String,
                i64,
            ) -> futures::future::LocalBoxFuture<
                'static,
                Result<AccessChangeOutcome, ApiRefusal>,
            > + 'static,
    {
        self.apply_then(change, send, || {});
    }

    /// [`Self::apply`], running `on_applied` once the change is accepted — for a form that clears
    /// itself only when what it described now exists.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn apply_then<F, D>(self, change: String, send: F, on_applied: D)
    where
        F: FnOnce(
                AuthStore,
                String,
                i64,
            ) -> futures::future::LocalBoxFuture<
                'static,
                Result<AccessChangeOutcome, ApiRefusal>,
            > + 'static,
        D: FnOnce() + 'static,
    {
        let (Some(event), Some(revision)) = (self.event_id.get_untracked(), self.revision()) else {
            return;
        };
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        leptos::task::spawn_local(async move {
            match send(self.store, event, revision).await {
                Ok(outcome) => {
                    self.adopt(change, outcome);
                    on_applied();
                }
                Err(refusal) => self.refuse(refusal),
            }
            self.busy.set(false);
        });
    }

    /// Show a change's answer: the new view, and the reservations it released or promoted named
    /// from the participants read before it.
    #[cfg(target_arch = "wasm32")]
    fn adopt(self, change: String, outcome: AccessChangeOutcome) {
        let before = self
            .participants
            .with_untracked(|p| p.loaded().cloned())
            .unwrap_or_default();
        let missions = self
            .missions
            .with_untracked(|m| m.loaded().cloned())
            .unwrap_or_default();
        let report = ChangeReport {
            released: describe_registrations(&outcome.released_registrations, &before, &missions),
            promoted: describe_registrations(&outcome.promoted_registrations, &before, &missions),
            change: change.clone(),
        };
        self.access.set(Loadable::Loaded(outcome.access));
        self.notice.set(Some(PanelNotice::Changed(report)));
        self.reload_participants();
        crate::v2::core::ui::toast::use_toasts().success(change);
    }

    /// Show a refused change; a stale revision reloads the view the change was prepared against.
    #[cfg(target_arch = "wasm32")]
    fn refuse(self, refusal: ApiRefusal) {
        let sentence = change_refusal_sentence(&refusal);
        crate::v2::core::ui::toast::use_toasts().error(sentence.clone());
        self.notice.set(Some(PanelNotice::Refused(sentence)));
        if is_revision_conflict(&refusal) {
            self.reload_access();
            self.reload_participants();
        }
    }
}

/// The operation's missions, each with its order of battle, in the operation's own order.
#[cfg(target_arch = "wasm32")]
async fn read_missions(
    store: AuthStore,
    event: &str,
) -> Result<Vec<MissionSeats>, crate::v2::core::api::client::ApiErr> {
    use crate::v2::core::api::client::api_get;
    use crate::v2::core::api::dto::{DataEnvelope, EventHub};
    use crate::v2::core::api::endpoints::encode_path_segment;
    let hub: EventHub = api_get(store, &format!("/events/{}", encode_path_segment(event))).await?;
    let mut missions = Vec::with_capacity(hub.missions.len());
    for mission in hub.missions {
        let path = format!(
            "/event-missions/{}/orbat",
            encode_path_segment(&mission.event_mission_id)
        );
        let orbat: DataEnvelope<OrbatSquad> = api_get(store, &path).await?;
        missions.push(MissionSeats {
            event_mission_id: mission.event_mission_id,
            title: mission.title,
            squads: orbat.data,
        });
    }
    Ok(missions)
}
