//! The deployment request form: an approved mission, optionally an event mission of an operation on
//! this server, and the refusal a request or cancellation came back with.
//!
//! **Role:** the control that opens the form, the choices it reads, the mission and event-mission
//! pickers, the deploy control, and the refusal panel with every seat and slot an ORBAT mismatch
//! names.
//! **Position:** the top of the deployments section of the selected server's card.
//! **Signals & state:** owns the chosen mission and event mission; reads and writes the
//! [`DeploymentPanel`].
//! **Invariants:** a deployment names the artifact the chosen mission's latest approval decided, so
//! only live missions with one are offered. An event mission is offered only for the chosen mission
//! and only from operations scheduled on this server; choosing none deploys without binding seats.
//! The choices are read when the form first opens.

use super::deployment_refusal::DeploymentRefusal;
use super::deployment_wording::DeployableMission;
use super::{ChoicesRead, DeploymentChoices, DeploymentPanel};
use crate::v2::core::api::dto::DeploymentRequest;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// Shared styling for the form's fields.
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// The request a choice of mission and event mission makes, or `None` until a mission is chosen.
pub(super) fn chosen_request(
    missions: &[DeployableMission],
    mission_id: &str,
    event_mission_id: &str,
) -> Option<DeploymentRequest> {
    let mission = missions.iter().find(|m| m.mission_id == mission_id)?;
    Some(DeploymentRequest {
        mission_id: mission.mission_id.clone(),
        artifact_id: mission.artifact_id.clone(),
        event_mission_id: (!event_mission_id.is_empty()).then(|| event_mission_id.to_string()),
    })
}

/// Read what a deployment of `server_id` can choose from: the library's live approved missions, and
/// the event missions of the operations scheduled on this server.
#[cfg(target_arch = "wasm32")]
pub(super) async fn read_choices(
    store: crate::v2::core::auth::AuthStore,
    server_id: &str,
) -> Result<DeploymentChoices, String> {
    use super::deployment_wording::{
        deployable_missions, event_mission_choices, operations_on_server,
    };
    use crate::v2::core::api::client::api_error_message;
    use crate::v2::core::api::endpoints::mission_deployments::{
        load_deployable_mission_choices, load_operation, load_upcoming_operations,
    };
    let library = load_deployable_mission_choices(store)
        .await
        .map_err(|e| api_error_message(&e, "The mission library could not be read"))?;
    let operations = load_upcoming_operations(store)
        .await
        .map_err(|e| api_error_message(&e, "The operation calendar could not be read"))?;
    let mut event_missions = Vec::new();
    for operation in operations_on_server(&operations.data, server_id) {
        let hub = load_operation(store, &operation.id)
            .await
            .map_err(|e| api_error_message(&e, "An operation on this server could not be read"))?;
        event_missions.extend(event_mission_choices(&hub));
    }
    Ok(DeploymentChoices {
        missions: deployable_missions(&library.data),
        event_missions,
    })
}

/// The request form of one server's deployments panel, and the last refusal.
pub(in super::super) fn deployment_request(panel: DeploymentPanel) -> impl IntoView {
    view! {
        <div class="space-y-3">
            {move || {
                if panel.form_open.get() {
                    request_form(panel).into_any()
                } else {
                    view! {
                        <button
                            type="button"
                            data-testid="deployment-request-open"
                            on:click=move |_| panel.open_form()
                            class="flex items-center gap-1.5 rounded-full bg-action px-4 py-2 text-label-md font-medium text-on-action transition hover:bg-action/90"
                        >
                            <MaterialIcon name="rocket_launch" class="text-[16px]" />
                            "Request a deployment"
                        </button>
                    }
                        .into_any()
                }
            }}
            {move || panel.refusal.get().map(refusal_panel)}
        </div>
    }
}

/// The open form: its choices as read, then the pickers and the deploy control.
fn request_form(panel: DeploymentPanel) -> impl IntoView {
    let mission = RwSignal::new(String::new());
    let event_mission = RwSignal::new(String::new());
    view! {
        <div class="space-y-3 rounded-xl border border-white/10 p-3 text-sm">
            {move || match panel.choices.get() {
                ChoicesRead::Loaded(choices) => pickers(panel, choices, mission, event_mission).into_any(),
                ChoicesRead::Failed(why) => view! { <p class="text-error-alert">{why}</p> }.into_any(),
                ChoicesRead::Loading | ChoicesRead::Idle => view! {
                    <p class="text-on-surface-variant">"Reading the live missions and this server's operations…"</p>
                }
                .into_any(),
            }}
            <button
                type="button"
                on:click=move |_| panel.form_open.set(false)
                class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface-variant hover:bg-white/5"
            >
                "Close the form"
            </button>
        </div>
    }
}

/// The mission and event-mission pickers and the deploy control.
fn pickers(
    panel: DeploymentPanel,
    choices: DeploymentChoices,
    mission: RwSignal<String>,
    event_mission: RwSignal<String>,
) -> impl IntoView {
    if choices.missions.is_empty() {
        return view! {
            <p class="text-on-surface-variant">
                "No live mission has an approved artifact to deploy. A mission becomes deployable once a reviewer approves its artifact."
            </p>
        }
        .into_any();
    }
    let choices = StoredValue::new(choices);
    let deploy = move |_| {
        let request = choices.with_value(|c| {
            chosen_request(
                &c.missions,
                &mission.get_untracked(),
                &event_mission.get_untracked(),
            )
        });
        if let Some(request) = request {
            panel.request(request);
        }
    };
    view! {
        <label class="flex flex-col gap-1">
            <span class="text-label-sm text-on-surface-variant">"Mission — live, with its approved artifact"</span>
            <select aria-label="Mission to deploy" class=FIELD
                on:change=move |ev| {
                    mission.set(event_target_value(&ev));
                    event_mission.set(String::new());
                }>
                <option value="">"Choose a mission…"</option>
                {choices.with_value(|c| {
                    c.missions
                        .iter()
                        .map(|m| view! { <option value=m.mission_id.clone()>{m.title.clone()}</option> })
                        .collect_view()
                })}
            </select>
        </label>
        <label class="flex flex-col gap-1">
            <span class="text-label-sm text-on-surface-variant">"Event mission — binds its seats to the artifact's slots"</span>
            <select aria-label="Event mission" class=FIELD
                prop:value=move || event_mission.get()
                on:change=move |ev| event_mission.set(event_target_value(&ev))>
                <option value="">"None — deploy without binding seats"</option>
                {move || {
                    let chosen = mission.get();
                    choices.with_value(|c| {
                        c.event_missions
                            .iter()
                            .filter(|em| em.mission_id == chosen)
                            .map(|em| view! { <option value=em.event_mission_id.clone()>{em.label.clone()}</option> })
                            .collect_view()
                    })
                }}
            </select>
        </label>
        <button
            type="button"
            data-testid="deployment-request-send"
            on:click=deploy
            prop:disabled=move || panel.busy.get() || mission.with(String::is_empty)
            class="flex items-center gap-1.5 rounded-full bg-action px-4 py-2 text-label-md font-medium text-on-action transition hover:bg-action/90 disabled:opacity-50"
        >
            <MaterialIcon name="rocket_launch" class="text-[16px]" />
            "Deploy"
        </button>
    }
    .into_any()
}

/// Why a request or cancellation was refused, with every seat and slot a mismatch names.
fn refusal_panel(refusal: DeploymentRefusal) -> impl IntoView {
    let groups = refusal.listed_details();
    view! {
        <div role="alert" data-testid="deployment-refusal"
            class="rounded-xl border border-error-alert/30 bg-error-alert/10 p-4 text-sm">
            <p class="text-on-surface">{refusal.sentence()}</p>
            {groups
                .into_iter()
                .map(|(heading, rows)| {
                    view! {
                        <p class="mt-3 text-xs font-semibold text-on-surface">{heading}</p>
                        <ul class="mt-1 max-h-40 list-disc overflow-y-auto pl-5 font-mono text-code-md text-error-alert">
                            {rows.into_iter().map(|row| view! { <li>{row}</li> }).collect_view()}
                        </ul>
                    }
                })
                .collect_view()}
        </div>
    }
}
