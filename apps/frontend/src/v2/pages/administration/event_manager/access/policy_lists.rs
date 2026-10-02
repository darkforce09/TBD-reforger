//! The Policies section: the operation's policy, and every squad's and slot's, with their editors.
//!
//! **Role:** renders the operation policy card and, for each mission, its squads and their slots —
//! each row stating where its policy comes from — with the controls that open a policy editor, save
//! a policy, and make a squad or slot inherit again; plus each mission's waiting-list promotion.
//! **Position:** the first section of the access panel.
//! **Signals & state:** owns which target's editor is open. The editor's draft is created when the
//! editor renders and is dropped with it; the open editor is shut whenever the access view's
//! revision moves — after a save, and after a stale-revision refusal reloads the view — so a draft
//! prepared against an older view is never saved over a newer one unseen.
//! **Invariants:** "inherit" and "admits nobody" are different controls. Inheriting removes a squad's
//! or slot's own policy (`DELETE`), so it follows its squad's or the operation's again; saving a
//! policy with no grants keeps an own policy that admits nobody. A new own policy starts from the
//! policy the target inherits now, so saving it unchanged changes nothing about who is admitted.
//! Every change goes through the panel's one change path, which names the revision.

use super::policy_editor::{policy_editor, PolicyDraft};
use super::policy_inheritance::{slot_origin, squad_origin, PolicyOrigin};
use super::state::{AccessPanel, Loadable, MissionSeats};
use super::waitlist_promotion::waitlist_promotion_button;
use crate::v2::core::api::dto::{EventAccessAdministration, EventAccessPolicy};
use leptos::prelude::*;

/// Whose policy an editor edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PolicyTarget {
    Operation,
    Squad {
        event_mission_id: String,
        faction: String,
        squad: String,
    },
    Slot {
        event_mission_id: String,
        faction: String,
        squad: String,
        slot_id: String,
    },
}

/// The success line a saved or removed policy is reported with.
pub(super) fn policy_change_label(target: &PolicyTarget, inherit: bool) -> String {
    match (target, inherit) {
        (PolicyTarget::Operation, _) => "Operation policy saved".to_string(),
        (PolicyTarget::Squad { faction, squad, .. }, false) => {
            format!("Policy of {faction} / {squad} saved")
        }
        (PolicyTarget::Squad { faction, squad, .. }, true) => {
            format!("{faction} / {squad} inherits the operation's policy again")
        }
        (PolicyTarget::Slot { .. }, false) => "Slot policy saved".to_string(),
        (PolicyTarget::Slot { .. }, true) => {
            "Slot policy removed; the slot inherits again".to_string()
        }
    }
}

/// The Policies section.
pub(super) fn policy_lists(panel: AccessPanel) -> impl IntoView {
    let editing = RwSignal::new(None::<PolicyTarget>);
    Effect::new(move |previous: Option<Option<i64>>| {
        let revision = panel
            .access
            .with(|access| access.loaded().map(|view| view.access_revision));
        if previous.is_some_and(|before| before != revision) {
            editing.set(None);
        }
        revision
    });
    move || {
        let Some(access) = panel.access.with(|a| a.loaded().cloned()) else {
            return ().into_any();
        };
        let access = StoredValue::new(access);
        view! {
            <div class="space-y-6">
                {operation_card(panel, access, editing)}
                {move || match panel.missions.get() {
                    Loadable::Loaded(missions) => missions
                        .into_iter()
                        .map(|mission| mission_block(panel, access, editing, mission))
                        .collect_view()
                        .into_any(),
                    Loadable::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
                    _ => view! { <p class="text-sm text-on-surface-variant">"Loading missions…"</p> }.into_any(),
                }}
            </div>
        }
        .into_any()
    }
}

/// The operation's own policy.
fn operation_card(
    panel: AccessPanel,
    access: StoredValue<EventAccessAdministration>,
    editing: RwSignal<Option<PolicyTarget>>,
) -> impl IntoView {
    let summary = access.with_value(|a| {
        if a.event_policy.grants.is_empty() {
            "Admits nobody: no account can see or join this operation except through a squad or \
             slot policy."
                .to_string()
        } else {
            super::policy_draft::policy_summary(&a.event_policy, &a.groups)
        }
    });
    view! {
        <section class="rounded-xl border border-white/10 p-4">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <div>
                    <h3 class="text-sm font-semibold text-on-surface">"Operation policy"</h3>
                    <p class="mt-1 text-sm text-on-surface-variant">{summary}</p>
                </div>
                {edit_button(editing, PolicyTarget::Operation, "Edit")}
            </div>
            {editor_slot(panel, access, editing, PolicyTarget::Operation)}
        </section>
    }
}

/// One mission: its squads and slots, and its waiting-list promotion.
fn mission_block(
    panel: AccessPanel,
    access: StoredValue<EventAccessAdministration>,
    editing: RwSignal<Option<PolicyTarget>>,
    mission: MissionSeats,
) -> impl IntoView {
    let emid = mission.event_mission_id.clone();
    view! {
        <section class="rounded-xl border border-white/10 p-4">
            <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
                <h3 class="text-sm font-semibold text-on-surface">{mission.title.clone()}</h3>
                {waitlist_promotion_button(panel, emid.clone())}
            </div>
            <div class="space-y-3">
                {mission
                    .squads
                    .into_iter()
                    .map(|squad| {
                        let target = PolicyTarget::Squad {
                            event_mission_id: emid.clone(),
                            faction: squad.faction.clone(),
                            squad: squad.squad.clone(),
                        };
                        let (own, line) = access.with_value(|a| {
                            let origin = squad_origin(a, &emid, &squad.faction, &squad.squad);
                            (origin.is_own(), origin.describe(a))
                        });
                        let slots = squad
                            .slots
                            .iter()
                            .map(|slot| {
                                let target = PolicyTarget::Slot {
                                    event_mission_id: emid.clone(),
                                    faction: squad.faction.clone(),
                                    squad: squad.squad.clone(),
                                    slot_id: slot.id.clone(),
                                };
                                let (own, line) = access.with_value(|a| {
                                    let origin = slot_origin(a, &emid, &squad.faction, &squad.squad, &slot.id);
                                    (origin.is_own(), origin.describe(a))
                                });
                                let name = format!("{}. {}", slot.number, slot.role);
                                target_row(panel, access, editing, target, name, line, own, "pl-6")
                            })
                            .collect_view();
                        let name = format!(
                            "{} / {}{}",
                            squad.faction,
                            squad.squad,
                            squad.callsign.as_deref().map(|c| format!(" ({c})")).unwrap_or_default()
                        );
                        view! {
                            <div class="rounded-lg bg-white/[0.02] p-2">
                                {target_row(panel, access, editing, target, name, line, own, "")}
                                <div class="mt-1 space-y-1">{slots}</div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </section>
    }
}

/// One squad or slot row: its name, where its policy comes from, and its controls.
#[allow(clippy::too_many_arguments)]
fn target_row(
    panel: AccessPanel,
    access: StoredValue<EventAccessAdministration>,
    editing: RwSignal<Option<PolicyTarget>>,
    target: PolicyTarget,
    name: String,
    origin_line: String,
    own: bool,
    indent: &'static str,
) -> impl IntoView {
    let inherit_target = target.clone();
    let inherit = move |_| {
        #[cfg(target_arch = "wasm32")]
        send_inheritance(panel, inherit_target.clone());
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &inherit_target;
    };
    view! {
        <div class=format!("{indent} py-1")>
            <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="min-w-0">
                    <p class="text-sm text-on-surface">{name}</p>
                    <p class="text-xs text-on-surface-variant">{origin_line}</p>
                </div>
                <div class="flex items-center gap-2">
                    {edit_button(editing, target.clone(), if own { "Edit own policy" } else { "Give it its own policy" })}
                    {own
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    on:click=inherit
                                    prop:disabled=move || panel.busy.get()
                                    title="Remove its own policy so it follows its squad's or the operation's again"
                                    class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface-variant transition hover:bg-white/5 disabled:opacity-50"
                                >
                                    "Inherit again"
                                </button>
                            }
                        })}
                </div>
            </div>
            {editor_slot(panel, access, editing, target)}
        </div>
    }
}

/// The button that opens `target`'s editor, or shuts it when it is the one open.
fn edit_button(
    editing: RwSignal<Option<PolicyTarget>>,
    target: PolicyTarget,
    label: &'static str,
) -> impl IntoView {
    let shown = target.clone();
    view! {
        <button
            type="button"
            on:click=move |_| {
                editing.update(|open| {
                    *open = if open.as_ref() == Some(&target) { None } else { Some(target.clone()) }
                })
            }
            class="rounded-full border border-white/10 px-3 py-1 text-xs text-primary transition hover:bg-white/5"
        >
            {move || if editing.with(|open| open.as_ref() == Some(&shown)) { "Close editor" } else { label }}
        </button>
    }
}

/// The policy an editor for `target` starts from: its own, or the one it inherits now.
fn starting_policy(access: &EventAccessAdministration, target: &PolicyTarget) -> EventAccessPolicy {
    let origin = match target {
        PolicyTarget::Operation => PolicyOrigin::Own(&access.event_policy),
        PolicyTarget::Squad {
            event_mission_id,
            faction,
            squad,
        } => squad_origin(access, event_mission_id, faction, squad),
        PolicyTarget::Slot {
            event_mission_id,
            faction,
            squad,
            slot_id,
        } => slot_origin(access, event_mission_id, faction, squad, slot_id),
    };
    origin.policy().clone()
}

/// The open editor under a row, when `target` is the one being edited.
fn editor_slot(
    panel: AccessPanel,
    access: StoredValue<EventAccessAdministration>,
    editing: RwSignal<Option<PolicyTarget>>,
    target: PolicyTarget,
) -> impl IntoView {
    move || {
        let open = editing.with(|open| open.as_ref() == Some(&target));
        open.then(|| {
            let (start, groups) =
                access.with_value(|a| (starting_policy(a, &target), a.groups.clone()));
            let draft = PolicyDraft::new(&start);
            let save_target = target.clone();
            let save = move |_| {
                #[cfg(target_arch = "wasm32")]
                send_policy(panel, save_target.clone(), draft);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&save_target, draft);
            };
            view! {
                <div class="mt-3 rounded-xl border border-primary/30 bg-surface/40 p-3">
                    {policy_editor(draft, groups)}
                    <div class="mt-3 flex justify-end gap-2">
                        <button
                            type="button"
                            on:click=move |_| editing.set(None)
                            class="rounded-full border border-white/10 px-4 py-1.5 text-sm text-on-surface-variant hover:bg-white/5"
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            on:click=save
                            prop:disabled=move || panel.busy.get()
                            class="rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50"
                        >
                            "Save policy"
                        </button>
                    </div>
                </div>
            }
        })
    }
}

/// Save the draft as `target`'s policy.
#[cfg(target_arch = "wasm32")]
fn send_policy(panel: AccessPanel, target: PolicyTarget, draft: PolicyDraft) {
    use crate::v2::core::api::dto::AccessPolicyChange;
    use crate::v2::core::api::endpoints::event_access_administration as routes;
    use futures::FutureExt;
    let policy = match draft.policy() {
        Ok(policy) => policy,
        Err(problem) => {
            crate::v2::core::ui::toast::use_toasts().error(problem);
            return;
        }
    };
    let label = policy_change_label(&target, false);
    panel.apply(label, move |store, event, revision| {
        async move {
            let change = AccessPolicyChange {
                expected_access_revision: revision,
                policy,
            };
            match &target {
                PolicyTarget::Operation => {
                    routes::put_event_access_policy(store, &event, &change).await
                }
                PolicyTarget::Squad {
                    event_mission_id,
                    faction,
                    squad,
                } => {
                    routes::put_squad_access_policy(
                        store,
                        event_mission_id,
                        faction,
                        squad,
                        &change,
                    )
                    .await
                }
                PolicyTarget::Slot {
                    event_mission_id,
                    slot_id,
                    ..
                } => {
                    routes::put_slot_access_policy(store, event_mission_id, slot_id, &change).await
                }
            }
        }
        .boxed_local()
    });
}

/// Remove `target`'s own policy, so it inherits again. The operation's policy has nothing to
/// inherit from, so it has no such control.
#[cfg(target_arch = "wasm32")]
fn send_inheritance(panel: AccessPanel, target: PolicyTarget) {
    use crate::v2::core::api::endpoints::event_access_administration as routes;
    use futures::FutureExt;
    let label = policy_change_label(&target, true);
    match target {
        PolicyTarget::Operation => {}
        PolicyTarget::Squad {
            event_mission_id,
            faction,
            squad,
        } => panel.apply(label, move |store, _event, revision| {
            async move {
                routes::remove_squad_access_policy(
                    store,
                    &event_mission_id,
                    &faction,
                    &squad,
                    revision,
                )
                .await
            }
            .boxed_local()
        }),
        PolicyTarget::Slot {
            event_mission_id,
            slot_id,
            ..
        } => panel.apply(label, move |store, _event, revision| {
            async move {
                routes::remove_slot_access_policy(store, &event_mission_id, &slot_id, revision)
                    .await
            }
            .boxed_local()
        }),
    }
}
