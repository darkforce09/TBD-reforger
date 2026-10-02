//! The Groups section: the operation's event groups, and the form that creates one.
//!
//! **Role:** renders the create form — a managed roster, or a partner guild with the roles its
//! members must hold — and one card per group with its provenance, roster and controls.
//! **Position:** the second section of the access panel.
//! **Signals & state:** owns the create form's fields and whether it is open; each card owns its
//! own. The section is rebuilt from every new access view, so a saved change shows at once.
//! **Invariants:** account ids are shown by name wherever the access view or the participant
//! evidence names them, and as the id itself otherwise. Every change goes through the panel's one
//! change path, which names the revision.

mod group_card;
mod group_fields;
pub(super) mod group_form;

use super::state::AccessPanel;
use crate::v2::core::api::dto::{EventAccessAdministration, ParticipantAccessExplanation};
use group_card::group_card;
use group_fields::group_fields;
use group_form::GroupForm;
use leptos::prelude::*;
use std::collections::HashMap;

/// Every account id the access view or the participant evidence names, with its display name.
pub(super) fn account_names(
    access: &EventAccessAdministration,
    participants: &[ParticipantAccessExplanation],
) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for entry in access.groups.iter().flat_map(|g| &g.roster) {
        names.insert(entry.discord_id.clone(), entry.username.clone());
    }
    for participant in participants {
        names.insert(participant.discord_id.clone(), participant.username.clone());
    }
    names
}

/// The Groups section.
pub(super) fn groups_section(panel: AccessPanel) -> impl IntoView {
    let creating = RwSignal::new(false);
    let form = RwSignal::new(GroupForm::blank());
    let create = move |_| {
        #[cfg(target_arch = "wasm32")]
        send_creation(panel, form, creating);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = form;
    };
    view! {
        <div class="space-y-4">
            <div class="rounded-xl border border-white/10 p-4">
                <div class="flex flex-wrap items-center justify-between gap-2">
                    <p class="text-sm text-on-surface-variant">
                        "A group is a named set of accounts a policy can admit: a roster you maintain, or the verified members of a partner guild."
                    </p>
                    <button
                        type="button"
                        on:click=move |_| creating.update(|open| *open = !*open)
                        class="rounded-full border border-white/10 px-3 py-1 text-xs text-primary hover:bg-white/5"
                    >
                        {move || if creating.get() { "Close" } else { "New group" }}
                    </button>
                </div>
                {move || {
                    creating
                        .get()
                        .then(|| {
                            view! {
                                <div class="mt-3 space-y-2">
                                    {group_fields(form)}
                                    <div class="flex justify-end">
                                        <button
                                            type="button"
                                            on:click=create
                                            prop:disabled=move || panel.busy.get()
                                            class="rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50"
                                        >
                                            "Create group"
                                        </button>
                                    </div>
                                </div>
                            }
                        })
                }}
            </div>
            {move || {
                let Some(access) = panel.access.with(|a| a.loaded().cloned()) else {
                    return ().into_any();
                };
                let participants = panel
                    .participants
                    .with(|p| p.loaded().cloned())
                    .unwrap_or_default();
                let names = StoredValue::new(account_names(&access, &participants));
                if access.groups.is_empty() {
                    return view! {
                        <p class="text-sm text-on-surface-variant">"This operation has no groups yet."</p>
                    }
                    .into_any();
                }
                access
                    .groups
                    .into_iter()
                    .map(|group| group_card(panel, group, names))
                    .collect_view()
                    .into_any()
            }}
        </div>
    }
}

/// Create the group the form describes, and close and clear the form when it was created.
#[cfg(target_arch = "wasm32")]
fn send_creation(panel: AccessPanel, form: RwSignal<GroupForm>, creating: RwSignal<bool>) {
    use crate::v2::core::api::dto::EventGroupCreation;
    use crate::v2::core::api::endpoints::event_access_administration::create_event_group;
    use futures::FutureExt;
    let fields = form.get_untracked();
    let (name, source) = match (fields.validated_name(), fields.validated_source()) {
        (Ok(name), Ok(source)) => (name, source),
        (Err(problem), _) | (_, Err(problem)) => {
            crate::v2::core::ui::toast::use_toasts().error(problem);
            return;
        }
    };
    let label = format!("Group {name} created");
    panel.apply_then(
        label,
        move |store, event, revision| {
            async move {
                let creation = EventGroupCreation {
                    expected_access_revision: revision,
                    name,
                    source,
                };
                create_event_group(store, &event, &creation).await
            }
            .boxed_local()
        },
        // A refusal leaves the form filled, so the operator can correct it.
        move || {
            form.set(GroupForm::blank());
            creating.set(false);
        },
    );
}
