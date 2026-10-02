//! The server's deployments: the one this panel is following, every deployment newest first, and the
//! detail of the one opened.
//!
//! **Role:** the followed deployment's progress — recorded, confirmed, failed or cancelled — then one
//! row per deployment with its state, mission, terrain, artifact and request time, and under an
//! opened row its detail: mission, artifact digest, document SHA-256, terrain and scenario,
//! transition, who requested it from where and when, deadline, state, bound seats, fleet command
//! and its state, the runtime session that confirmed it, when it finished and why it failed — with
//! a cancel control while it is in flight.
//! **Position:** under the request form in the deployments section of the selected server's card.
//! **Signals & state:** reads the [`DeploymentPanel`]; opens a row's detail and cancels through it.
//! **Invariants:** only a deployment in flight offers a cancel control, and the backend still
//! refuses one whose command an executor has taken up; that refusal is shown, not hidden.

use super::deployment_wording::{detail_rows, in_flight, state_label, state_tone, summary_line};
use super::{DeploymentList, DeploymentPanel};
use crate::v2::core::api::dto::MissionDeployment;
use crate::v2::core::ui::badge_class;
use leptos::prelude::*;

/// The followed deployment, then the list.
pub(in super::super) fn deployment_list(panel: DeploymentPanel) -> impl IntoView {
    let me = StoredValue::new(panel.store.user.get_untracked().map(|u| u.discord_id));
    view! {
        <div class="space-y-4">
            {move || panel.followed.get().map(followed_panel)}
            <div class="flex items-center justify-between">
                <h4 class="font-mono text-label-sm tracking-widest text-outline uppercase">"Deployments"</h4>
                <button type="button"
                    class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface-variant hover:bg-white/5"
                    on:click=move |_| panel.reload()>
                    "Refresh"
                </button>
            </div>
            {move || match panel.list.get() {
                DeploymentList::Loaded(deployments) if deployments.is_empty() => view! {
                    <p class="text-sm text-on-surface-variant">"Nothing has been deployed to this server yet."</p>
                }
                .into_any(),
                DeploymentList::Loaded(deployments) => view! {
                    <ul class="space-y-2" data-testid="mission-deployments">
                        {deployments
                            .into_iter()
                            .map(|deployment| deployment_row(panel, deployment, me))
                            .collect_view()}
                    </ul>
                }
                .into_any(),
                DeploymentList::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
                DeploymentList::Loading | DeploymentList::Idle => view! {
                    <p class="text-sm text-on-surface-variant">"Loading the deployments…"</p>
                }
                .into_any(),
            }}
        </div>
    }
}

/// The deployment this panel requested: still in flight, or how it ended.
fn followed_panel(deployment: MissionDeployment) -> impl IntoView {
    let (tone, text) = match deployment.state.as_str() {
        "requested" => (
            "border-primary/30 bg-primary/10",
            format!(
                "{} is recorded — its fleet command is {}; waiting for a runtime session to confirm the artifact by {}.",
                deployment.mission_title,
                super::super::fleet_commands::command_wording::state_label(&deployment.fleet_command_state)
                    .to_lowercase(),
                crate::v2::core::utils::utc_timestamp::utc_label(&deployment.deadline_at)
            ),
        ),
        "confirmed" => (
            "border-success/30 bg-success/10",
            format!("{} is confirmed: a runtime session reported the artifact.", deployment.mission_title),
        ),
        "failed" => (
            "border-error-alert/30 bg-error-alert/10",
            format!(
                "The deployment of {} failed: {}",
                deployment.mission_title,
                deployment.failure_reason.clone().unwrap_or_default()
            ),
        ),
        _ => (
            "border-white/10 bg-white/5",
            format!("The deployment of {} is {}.", deployment.mission_title, state_label(&deployment.state).to_lowercase()),
        ),
    };
    view! {
        <div role="status" data-testid="deployment-followed"
            class=format!("rounded-xl border px-4 py-3 text-sm text-on-surface {tone}")>
            <p class="font-mono text-label-sm tracking-widest text-outline uppercase">"Your deployment"</p>
            <p class="mt-1">{text}</p>
        </div>
    }
}

/// One deployment, and its detail while it is open.
fn deployment_row(
    panel: DeploymentPanel,
    deployment: MissionDeployment,
    me: StoredValue<Option<String>>,
) -> impl IntoView {
    let id = StoredValue::new(deployment.id.clone());
    let open = move || panel.selected.get().as_deref() == Some(id.get_value().as_str());
    let toggle = move |_| {
        let this = id.get_value();
        panel.selected.update(|selected| {
            *selected = if selected.as_deref() == Some(this.as_str()) {
                None
            } else {
                Some(this)
            };
        });
    };
    let cancellable = in_flight(&deployment.state);
    let rows = detail_rows(&deployment, me.get_value().as_deref());
    view! {
        <li class="rounded-xl border border-white/10 text-sm">
            <button type="button" on:click=toggle aria-expanded=move || open().to_string()
                class="flex w-full flex-wrap items-center justify-between gap-2 p-3 text-left hover:bg-white/[0.03]">
                <span class="flex flex-wrap items-center gap-2 text-on-surface">
                    <span class=badge_class(state_tone(&deployment.state))>{state_label(&deployment.state)}</span>
                    <span class="font-medium">{deployment.mission_title.clone()}</span>
                </span>
                <span class="font-mono text-code-md text-outline">{summary_line(&deployment)}</span>
            </button>
            {move || {
                open()
                    .then(|| {
                        let rows = rows.clone();
                        view! {
                            <div class="border-t border-white/10 p-3">
                                <dl class="grid grid-cols-1 gap-x-4 gap-y-1 sm:grid-cols-[11rem_1fr]">
                                    {rows
                                        .into_iter()
                                        .map(|(label, value)| {
                                            view! {
                                                <dt class="font-mono text-label-sm tracking-wider text-outline uppercase">{label}</dt>
                                                <dd class="min-w-0 break-all font-mono text-code-md text-on-surface">{value}</dd>
                                            }
                                        })
                                        .collect_view()}
                                </dl>
                                {cancellable
                                    .then(|| {
                                        view! {
                                            <button type="button"
                                                class="mt-3 rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10 disabled:opacity-50"
                                                prop:disabled=move || panel.busy.get()
                                                on:click=move |_| panel.cancel(id.get_value())>
                                                "Cancel this deployment"
                                            </button>
                                        }
                                    })}
                            </div>
                        }
                    })
            }}
        </li>
    }
}
