//! The grants-and-conditions editor every access policy is edited in.
//!
//! **Role:** renders a policy draft as its alternatives — each grant a card of conditions that must
//! all hold — with the controls that add and remove grants and conditions, pick a condition's kind
//! and fill its fields: a guild and role for a Discord role, a group for an event group, and an
//! account, found through the member directory, for a named account.
//! **Position:** opened inline under the operation policy, a squad row or a slot row of the access
//! panel's policy lists.
//! **Signals & state:** the draft is one signal holding the plain rows of [`super::policy_draft`]
//! and the id counter that keys them. It is created when the editor renders, so it belongs to the
//! editor and is disposed with it.
//! **Invariants:** rows are keyed by id — and a condition row by its kind as well — so typing into a
//! field never rebuilds the row it is typing into, while a kind change rebuilds that one row with the
//! fields the new kind needs. An empty grant list is shown as what it is — a policy that admits
//! nobody — rather than as an unfinished form.

use super::member_search::MemberSearch;
use super::policy_draft::{
    add_condition, add_grant, draft_from_policy, policy_from_draft, remove_condition, remove_grant,
    set_field, set_kind, ConditionFields, ConditionIdentifier, ConditionKind, GrantFields,
};
use crate::v2::core::api::dto::{EventAccessPolicy, EventGroupSource, EventGroupView, Member};
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// Shared styling for the editor's text fields.
const FIELD: &str = "min-w-0 flex-1 rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 font-mono text-sm text-on-surface outline-none focus:border-primary/60";

/// An editable copy of a policy.
#[derive(Clone, Copy)]
pub(super) struct PolicyDraft {
    grants: RwSignal<Vec<GrantFields>>,
    ids: StoredValue<u64>,
}

impl PolicyDraft {
    /// A draft of `policy`, owned by the reactive scope that creates it.
    pub(super) fn new(policy: &EventAccessPolicy) -> Self {
        let mut ids = 0;
        let grants = draft_from_policy(policy, &mut ids);
        Self {
            grants: RwSignal::new(grants),
            ids: StoredValue::new(ids),
        }
    }

    /// Apply one structural or field edit.
    fn edit(self, change: impl FnOnce(&mut Vec<GrantFields>, &mut u64)) {
        let mut ids = self.ids.get_value();
        self.grants.update(|grants| change(grants, &mut ids));
        self.ids.set_value(ids);
    }

    /// The draft read back into a policy, or what is wrong with it.
    pub(super) fn policy(self) -> Result<EventAccessPolicy, String> {
        self.grants
            .with_untracked(|grants| policy_from_draft(grants))
    }

    /// Whether the draft has no grants — a policy that admits nobody.
    pub(super) fn admits_nobody(self) -> bool {
        self.grants.with(Vec::is_empty)
    }

    fn field(self, condition: u64, field: ConditionIdentifier) -> String {
        self.grants.with(|grants| {
            grants
                .iter()
                .flat_map(|g| &g.conditions)
                .find(|c| c.id == condition)
                .map(|c| c.field(field).to_string())
                .unwrap_or_default()
        })
    }

    fn set(self, condition: u64, field: ConditionIdentifier, value: String) {
        self.edit(|grants, _| set_field(grants, condition, field, value));
    }

    fn conditions(self, grant: u64) -> Vec<ConditionFields> {
        self.grants.with(|grants| {
            grants
                .iter()
                .find(|g| g.id == grant)
                .map(|g| g.conditions.clone())
                .unwrap_or_default()
        })
    }

    fn position(self, grant: u64) -> usize {
        self.grants
            .with(|grants| grants.iter().position(|g| g.id == grant))
            .map_or(0, |i| i + 1)
    }
}

/// The editor for one draft. `groups` are the operation's event groups, which a group condition
/// picks from and whose partner guilds a role condition is offered.
pub(super) fn policy_editor(draft: PolicyDraft, groups: Vec<EventGroupView>) -> impl IntoView {
    let groups = StoredValue::new(groups);
    view! {
        <div class="space-y-3" data-testid="policy-editor">
            <p class="text-xs text-on-surface-variant">
                "Any one grant admits an account; every condition inside a grant must hold."
            </p>
            {move || {
                draft
                    .admits_nobody()
                    .then(|| {
                        view! {
                            <p class="rounded-lg border border-tactical-yellow/40 bg-tactical-yellow/10 px-3 py-2 text-sm text-tactical-yellow">
                                "No grants: saved like this, the policy admits nobody. That closes every seat it decides — it does not inherit."
                            </p>
                        }
                    })
            }}
            <For each=move || draft.grants.get() key=|grant| grant.id let:grant>
                {grant_card(draft, grant.id, groups)}
            </For>
            <button
                type="button"
                on:click=move |_| draft.edit(add_grant)
                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-1.5 text-sm text-on-surface transition hover:bg-white/5"
            >
                <MaterialIcon name="add" class="text-base" />
                "Add an alternative grant"
            </button>
        </div>
    }
}

/// One grant: its number, its conditions, and the controls that change them.
fn grant_card(
    draft: PolicyDraft,
    grant: u64,
    groups: StoredValue<Vec<EventGroupView>>,
) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] p-3">
            <div class="mb-2 flex items-center justify-between">
                <span class="font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                    {move || format!("Grant {} — all of", draft.position(grant))}
                </span>
                <button
                    type="button"
                    on:click=move |_| draft.edit(|grants, _| remove_grant(grants, grant))
                    class="text-xs text-error-alert hover:underline"
                >
                    "Remove grant"
                </button>
            </div>
            <div class="space-y-2">
                <For
                    each=move || draft.conditions(grant)
                    key=|condition| (condition.id, condition.kind)
                    let:condition
                >
                    {condition_row(draft, grant, condition.id, condition.kind, groups)}
                </For>
            </div>
            <button
                type="button"
                on:click=move |_| draft.edit(|grants, ids| add_condition(grants, grant, ids))
                class="mt-2 text-xs text-primary hover:underline"
            >
                "+ Add a condition"
            </button>
        </div>
    }
}

/// One condition: its kind, the fields that kind needs, and its removal.
fn condition_row(
    draft: PolicyDraft,
    grant: u64,
    condition: u64,
    kind: ConditionKind,
    groups: StoredValue<Vec<EventGroupView>>,
) -> impl IntoView {
    let text_field = move |field: ConditionIdentifier, label: &'static str| {
        view! {
            <input
                aria-label=label
                placeholder=label
                prop:value=move || draft.field(condition, field)
                on:input=move |ev| draft.set(condition, field, event_target_value(&ev))
                class=FIELD
            />
        }
    };
    let fields = match kind {
        ConditionKind::DiscordRole => {
            let partner_guilds: Vec<String> = groups.with_value(|groups| {
                groups
                    .iter()
                    .filter_map(|g| match &g.source {
                        EventGroupSource::PartnerGuild { guild_id, .. } => Some(guild_id.clone()),
                        EventGroupSource::ManagedRoster {} => None,
                    })
                    .collect()
            });
            let list = format!("partner-guilds-{condition}");
            view! {
                <input
                    aria-label="Guild id"
                    placeholder="Guild id"
                    list=list.clone()
                    prop:value=move || draft.field(condition, ConditionIdentifier::Guild)
                    on:input=move |ev| draft.set(condition, ConditionIdentifier::Guild, event_target_value(&ev))
                    class=FIELD
                />
                <datalist id=list>
                    {partner_guilds.into_iter().map(|g| view! { <option value=g></option> }).collect_view()}
                </datalist>
                {text_field(ConditionIdentifier::Role, "Role id")}
            }
            .into_any()
        }
        ConditionKind::EventGroup => {
            let chosen = group_of(draft, condition);
            let options = groups.with_value(|groups| {
                groups
                    .iter()
                    .map(|g| (g.id.clone(), g.name.clone()))
                    .collect::<Vec<_>>()
            });
            view! {
                <select
                    aria-label="Event group"
                    on:change=move |ev| draft.set(condition, ConditionIdentifier::Group, event_target_value(&ev))
                    class=FIELD
                >
                    <option value="" selected=chosen.is_empty()>"Choose a group"</option>
                    {options
                        .into_iter()
                        .map(|(id, name)| {
                            let selected = id == chosen;
                            view! { <option value=id selected=selected>{name}</option> }
                        })
                        .collect_view()}
                </select>
            }
            .into_any()
        }
        ConditionKind::NamedAccount => {
            let pick = Callback::new(move |member: Member| {
                draft.set(condition, ConditionIdentifier::Account, member.discord_id)
            });
            view! {
                <div class="flex w-full flex-col gap-2">
                    {text_field(ConditionIdentifier::Account, "Discord account id")}
                    <MemberSearch on_pick=pick label="Find the account" />
                </div>
            }
            .into_any()
        }
        ConditionKind::Authenticated | ConditionKind::TbdMember => ().into_any(),
    };
    view! {
        <div class="flex flex-wrap items-center gap-2 rounded-lg bg-surface/60 p-2">
            <select
                aria-label="Condition kind"
                on:change=move |ev| {
                    if let Some(kind) = ConditionKind::from_wire(&event_target_value(&ev)) {
                        draft.edit(|grants, _| set_kind(grants, condition, kind));
                    }
                }
                class="rounded-md border border-outline-variant/40 bg-surface px-2 py-1.5 text-sm text-on-surface outline-none"
            >
                {ConditionKind::ALL
                    .into_iter()
                    .map(|option| {
                        let chosen = option == kind;
                        view! {
                            <option value=option.wire() selected=chosen>
                                {option.label()}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            {fields}
            <button
                type="button"
                aria-label="Remove condition"
                title="Remove condition"
                on:click=move |_| draft.edit(|grants, _| remove_condition(grants, grant, condition))
                class="ml-auto flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
            >
                <MaterialIcon name="close" class="text-base" />
            </button>
        </div>
    }
}

/// A condition's group id, read without subscribing: the group select is built once per row.
fn group_of(draft: PolicyDraft, condition: u64) -> String {
    draft.grants.with_untracked(|grants| {
        grants
            .iter()
            .flat_map(|g| &g.conditions)
            .find(|c| c.id == condition)
            .map(|c| c.group_id.clone())
            .unwrap_or_default()
    })
}
