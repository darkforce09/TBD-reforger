//! One server's mission deployments: the list and each one's detail, a request that runs an approved
//! artifact, following it to confirmation, and cancelling one still queued.
//!
//! **Role:** declares the list with its detail, the request form, the refusal and the wording under
//! them, and holds the panel's state with its operations — read the list, read the choices a
//! request offers, request, follow and cancel.
//! **Position:** a section of the selected server's card on the server control screen.
//! **Signals & state:** [`DeploymentPanel`] is one copyable handle, created by the server card for
//! the server it shows: the list as read, the deployment whose detail is open, the deployment this
//! panel requested and is following, whether the request form is open with its choices, the
//! in-flight flag, the last refusal, and a follow generation that retires a superseded or unmounted
//! follow.
//! **Invariants:** a request's 202 records a deployment and nothing more; it is followed by reading
//! `GET /servers/:id/deployments/:deploymentId` every two seconds until it is confirmed, failed or
//! cancelled, and only then announced. The choices a request offers are read when the form opens,
//! never before. Every request is browser-only; a native build keeps the list idle.

mod deployment_list;
mod deployment_refusal;
mod deployment_request;
pub(super) mod deployment_wording;

pub(super) use deployment_list::deployment_list;
pub(super) use deployment_request::deployment_request;

use crate::v2::core::api::dto::{DeploymentRequest, MissionDeployment};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::toast::Toasts;
use deployment_refusal::DeploymentRefusal;
use deployment_wording::{DeployableMission, EventMissionChoice};
use leptos::prelude::*;

/// How long the panel waits between two reads of a followed deployment.
const FOLLOW_INTERVAL_MS: u32 = 2_000;

/// How many reads of a followed deployment may fail in a row before the panel stops following it.
const FOLLOW_READ_FAILURES: u32 = 5;

/// The deployment list as read: not yet, in flight, refused with a reason, or answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum DeploymentList {
    Idle,
    Loading,
    Failed(String),
    Loaded(Vec<MissionDeployment>),
}

/// What a deployment request of this server can choose from.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct DeploymentChoices {
    pub(super) missions: Vec<DeployableMission>,
    pub(super) event_missions: Vec<EventMissionChoice>,
}

/// The choices as read: not yet, in flight, refused with a reason, or answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ChoicesRead {
    Idle,
    Loading,
    Failed(String),
    Loaded(DeploymentChoices),
}

/// Every signal the deployments panel runs on.
#[derive(Clone, Copy)]
pub(super) struct DeploymentPanel {
    pub(super) store: AuthStore,
    pub(super) toasts: Toasts,
    pub(super) server_id: StoredValue<String>,
    pub(super) list: RwSignal<DeploymentList>,
    /// The deployment whose detail is open.
    pub(super) selected: RwSignal<Option<String>>,
    /// The deployment this panel requested, as last read.
    pub(super) followed: RwSignal<Option<MissionDeployment>>,
    pub(super) form_open: RwSignal<bool>,
    pub(super) choices: RwSignal<ChoicesRead>,
    pub(super) busy: RwSignal<bool>,
    pub(super) refusal: RwSignal<Option<DeploymentRefusal>>,
    follow_generation: StoredValue<u64>,
}

impl DeploymentPanel {
    /// The panel of one server, reading its deployments at once.
    pub(super) fn new(store: AuthStore, toasts: Toasts, server_id: String) -> Self {
        let panel = Self {
            store,
            toasts,
            server_id: StoredValue::new(server_id),
            list: RwSignal::new(DeploymentList::Idle),
            selected: RwSignal::new(None),
            followed: RwSignal::new(None),
            form_open: RwSignal::new(false),
            choices: RwSignal::new(ChoicesRead::Idle),
            busy: RwSignal::new(false),
            refusal: RwSignal::new(None),
            follow_generation: StoredValue::new(0),
        };
        panel.reload();
        panel
    }

    /// The runtime session that confirmed the server's newest confirmed deployment.
    pub(super) fn latest_confirmed_session(self) -> Option<String> {
        match self.list.get() {
            DeploymentList::Loaded(deployments) => {
                deployment_wording::latest_confirmed_session(&deployments)
            }
            _ => None,
        }
    }

    /// Read the list again. A list already on screen stays there until the new one lands.
    pub(super) fn reload(self) {
        let _ = self.list.try_update(|list| {
            if !matches!(list, DeploymentList::Loaded(_)) {
                *list = DeploymentList::Loading;
            }
        });
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            use crate::v2::core::api::endpoints::mission_deployments::load_mission_deployments;
            let Some(server) = self.server_id.try_get_value() else {
                return;
            };
            let read = load_mission_deployments(self.store, &server).await;
            let _ = self.list.try_set(match read {
                Ok(page) => DeploymentList::Loaded(page.items),
                Err(e) => DeploymentList::Failed(crate::v2::core::api::client::api_error_message(
                    &e,
                    "The deployments could not be read",
                )),
            });
        });
    }

    /// Open the request form, reading its choices the first time.
    pub(super) fn open_form(self) {
        self.form_open.set(true);
        self.refusal.set(None);
        if matches!(
            self.choices.get_untracked(),
            ChoicesRead::Loaded(_) | ChoicesRead::Loading
        ) {
            return;
        }
        self.choices.set(ChoicesRead::Loading);
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            let Some(server) = self.server_id.try_get_value() else {
                return;
            };
            let read = deployment_request::read_choices(self.store, &server).await;
            let _ = self.choices.try_set(match read {
                Ok(choices) => ChoicesRead::Loaded(choices),
                Err(why) => ChoicesRead::Failed(why),
            });
        });
    }

    /// Request a deployment, then follow it to confirmation.
    pub(super) fn request(self, request: DeploymentRequest) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::endpoints::mission_deployments::request_mission_deployment;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match request_mission_deployment(self.store, &server, &request).await {
                    Ok(deployment) => {
                        self.toasts.message(format!(
                            "Deployment of {} recorded — waiting for a runtime session to confirm it",
                            deployment.mission_title
                        ));
                        let id = deployment.id.clone();
                        let _ = self.selected.try_set(Some(id.clone()));
                        let _ = self.followed.try_set(Some(deployment));
                        let _ = self.form_open.try_set(false);
                        self.reload();
                        self.follow(id);
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(DeploymentRefusal::from_refusal(
                            &refused,
                            "The deployment could not be requested",
                        )));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = request;
    }

    /// Follow one deployment until it is confirmed, failed or cancelled, then announce how it ended.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn follow(self, deployment_id: String) {
        use crate::v2::core::api::endpoints::mission_deployments::load_mission_deployment;
        use deployment_wording::announce_deployment;
        let generation = self.follow_generation.get_value().wrapping_add(1);
        self.follow_generation.set_value(generation);
        leptos::task::spawn_local(async move {
            let mut failures = 0;
            loop {
                gloo_timers::future::TimeoutFuture::new(FOLLOW_INTERVAL_MS).await;
                if self.follow_generation.try_get_value() != Some(generation) {
                    return;
                }
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match load_mission_deployment(self.store, &server, &deployment_id).await {
                    Ok(deployment) => {
                        failures = 0;
                        if self.followed.try_set(Some(deployment.clone())).is_some() {
                            return;
                        }
                        if announce_deployment(&deployment, &self.toasts) {
                            self.reload();
                            return;
                        }
                    }
                    Err((401, _)) => return,
                    Err(e) => {
                        failures += 1;
                        if failures >= FOLLOW_READ_FAILURES {
                            self.toasts.error(format!(
                                "Stopped following the deployment: {}",
                                crate::v2::core::api::client::api_error_message(
                                    &e,
                                    "it could not be read"
                                )
                            ));
                            return;
                        }
                    }
                }
            }
        });
    }

    /// Cancel a deployment whose command no executor has taken up.
    pub(super) fn cancel(self, deployment_id: String) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::endpoints::mission_deployments::cancel_mission_deployment;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match cancel_mission_deployment(self.store, &server, &deployment_id).await {
                    Ok(deployment) => {
                        let followed = self
                            .followed
                            .try_get_untracked()
                            .flatten()
                            .is_some_and(|f| f.id == deployment.id);
                        if followed {
                            // The cancellation is its outcome; the follow stands down.
                            if let Some(generation) = self.follow_generation.try_get_value() {
                                self.follow_generation.set_value(generation.wrapping_add(1));
                            }
                            let _ = self.followed.try_set(Some(deployment.clone()));
                        }
                        self.toasts.message(format!(
                            "The deployment of {} is cancelled — its command never ran",
                            deployment.mission_title
                        ));
                        self.reload();
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(DeploymentRefusal::from_refusal(
                            &refused,
                            "The deployment could not be cancelled",
                        )));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = deployment_id;
    }
}

#[cfg(test)]
#[path = "tests/mission_deployments.rs"]
mod tests;
