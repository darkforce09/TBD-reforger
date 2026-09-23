//! One event group: what it is, who made it, who is in it, and the controls that change it.
//!
//! **Role:** renders a group's name, source and provenance; its edit form (name, and the roster or
//! partner-guild source); its deletion behind an inline confirmation; and — for a managed roster —
//! its members, each with who added them and when, a remove control, and the member search that
//! adds one.
//! **Position:** repeated down the Groups section of the access panel.
//! **Signals & state:** owns whether the edit form and the delete confirmation are open, and the
//! edit form's fields; every change goes through the panel's one change path.
//! **Invariants:** a partner-guild group has no roster to edit: its membership comes only from
//! bot-verified Discord observations, and the card says so instead of offering controls the backend
//! would refuse. Deleting a group some policy still names is refused by the backend, and that
//! refusal is shown as the backend words it. An edit sends only the fields that changed.

use super::super::member_search::MemberSearch;
use super::super::state::AccessPanel;
use super::group_fields::group_fields;
use super::group_form::{authorship_line, provenance_line, source_line, GroupForm};
use crate::v2::core::api::dto::{EventGroupSource, EventGroupView, Member};
use leptos::prelude::*;
use std::collections::HashMap;

/// One group's card. `names` turns account ids into display names where they are known.
pub(super) fn group_card(
    panel: AccessPanel,
    group: EventGroupView,
    names: StoredValue<HashMap<String, String>>,
) -> impl IntoView {
    let name_of =
        move |id: &str| names.with_value(|n| n.get(id).cloned().unwrap_or_else(|| id.to_string()));
    let editing = RwSignal::new(false);
    let confirming_delete = RwSignal::new(false);
    let form = RwSignal::new(GroupForm::of(&group.name, &group.source));
    let group = StoredValue::new(group);
    let (title, source, provenance, managed) = group.with_value(|g| {
        (
            g.name.clone(),
            source_line(&g.source),
            provenance_line(&g.provenance, name_of),
            g.source == EventGroupSource::ManagedRoster {},
        )
    });

    let save = move |_| {
        #[cfg(target_arch = "wasm32")]
        send_edit(panel, group.get_value(), form.get_untracked());
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (group, form);
    };
    let delete = move |_| {
        confirming_delete.set(false);
        #[cfg(target_arch = "wasm32")]
        send_delete(panel, group.with_value(|g| (g.id.clone(), g.name.clone())));
    };

    view! {
        <article class="rounded-xl border border-white/10 p-4" data-testid="event-group">
            <div class="flex flex-wrap items-start justify-between gap-2">
                <div class="min-w-0">
                    <h3 class="text-sm font-semibold text-on-surface">{title.clone()}</h3>
                    <p class="mt-1 text-xs text-on-surface-variant">{source}</p>
                    <p class="text-xs text-on-surface-variant">{provenance}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        type="button"
                        on:click=move |_| editing.update(|open| *open = !*open)
                        class="rounded-full border border-white/10 px-3 py-1 text-xs text-primary hover:bg-white/5"
                    >
                        {move || if editing.get() { "Close editor" } else { "Edit" }}
                    </button>
                    <button
                        type="button"
                        on:click=move |_| confirming_delete.set(true)
                        class="rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10"
                    >
                        "Delete"
                    </button>
                </div>
            </div>
            {move || {
                confirming_delete
                    .get()
                    .then(|| {
                        view! {
                            <div class="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-error-alert/30 bg-error-alert/10 px-3 py-2 text-sm">
                                <span class="text-error-alert">
                                    "Delete this group? A group that any policy still names cannot be deleted until every policy drops it."
                                </span>
                                <span class="flex gap-2">
                                    <button
                                        type="button"
                                        on:click=move |_| confirming_delete.set(false)
                                        class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface-variant"
                                    >
                                        "Keep"
                                    </button>
                                    <button
                                        type="button"
                                        on:click=delete
                                        prop:disabled=move || panel.busy.get()
                                        class="rounded-full bg-error-alert/20 px-3 py-1 text-xs text-error-alert disabled:opacity-50"
                                    >
                                        "Delete group"
                                    </button>
                                </span>
                            </div>
                        }
                    })
            }}
            {move || {
                editing
                    .get()
                    .then(|| {
                        view! {
                            <div class="mt-3 space-y-2 rounded-lg bg-surface/40 p-3">
                                {group_fields(form)}
                                <div class="flex justify-end">
                                    <button
                                        type="button"
                                        on:click=save
                                        prop:disabled=move || panel.busy.get()
                                        class="rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50"
                                    >
                                        "Save group"
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
            {if managed {
                roster(panel, group, name_of).into_any()
            } else {
                view! {
                    <p class="mt-3 text-xs text-on-surface-variant">
                        "Membership follows bot-verified Discord observations of the partner guild; there is no roster to edit."
                    </p>
                }
                .into_any()
            }}
        </article>
    }
}

/// A managed roster: its members with their provenance, and the search that adds one.
fn roster(
    panel: AccessPanel,
    group: StoredValue<EventGroupView>,
    name_of: impl Fn(&str) -> String + Copy + 'static,
) -> impl IntoView {
    let entries = group.with_value(|g| g.roster.clone());
    let count = entries.len();
    let add = Callback::new(move |member: Member| {
        #[cfg(target_arch = "wasm32")]
        send_member(panel, group.with_value(|g| g.id.clone()), member, true);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (group, member);
    });
    view! {
        <div class="mt-3">
            <p class="mb-1 font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                {format!("Roster · {count}")}
            </p>
            <ul class="divide-y divide-white/5 rounded-lg border border-white/10">
                {entries
                    .into_iter()
                    .map(|entry| {
                        let added = authorship_line(
                            entry.added_by.as_deref(),
                            entry.system_origin.as_deref(),
                            &entry.added_at,
                            name_of,
                        );
                        let member = Member {
                            discord_id: entry.discord_id.clone(),
                            username: entry.username.clone(),
                            avatar_url: None,
                        };
                        let remove = move |_| {
                            #[cfg(target_arch = "wasm32")]
                            send_member(panel, group.with_value(|g| g.id.clone()), member.clone(), false);
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &member;
                        };
                        view! {
                            <li class="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm">
                                <span class="min-w-0">
                                    <span class="text-on-surface">{entry.username.clone()}</span>
                                    <span class="ml-2 font-mono text-xs text-on-surface-variant">
                                        {entry.discord_id.clone()}
                                    </span>
                                    <span class="block text-xs text-on-surface-variant">
                                        {format!("Added by {added}")}
                                    </span>
                                </span>
                                <button
                                    type="button"
                                    on:click=remove
                                    prop:disabled=move || panel.busy.get()
                                    class="rounded-full border border-white/10 px-3 py-1 text-xs text-error-alert disabled:opacity-50"
                                >
                                    "Remove"
                                </button>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <div class="mt-2">
                <MemberSearch on_pick=add label="Add a member to this roster" />
            </div>
        </div>
    }
}

/// Save the edit form's changes to the group.
#[cfg(target_arch = "wasm32")]
fn send_edit(panel: AccessPanel, group: EventGroupView, form: GroupForm) {
    use crate::v2::core::api::dto::EventGroupChange;
    use crate::v2::core::api::endpoints::event_access_administration::change_event_group;
    use futures::FutureExt;
    let (name, source) = match form.changes_from(&group.name, &group.source) {
        Ok(changes) => changes,
        Err(problem) => {
            crate::v2::core::ui::toast::use_toasts().error(problem);
            return;
        }
    };
    if name.is_none() && source.is_none() {
        crate::v2::core::ui::toast::use_toasts().message("No changes to save");
        return;
    }
    let label = format!("Group {} saved", name.as_deref().unwrap_or(&group.name));
    panel.apply(label, move |store, event, revision| {
        async move {
            let change = EventGroupChange {
                expected_access_revision: revision,
                name,
                source,
            };
            change_event_group(store, &event, &group.id, &change).await
        }
        .boxed_local()
    });
}

/// Delete the group.
#[cfg(target_arch = "wasm32")]
fn send_delete(panel: AccessPanel, (group, name): (String, String)) {
    use crate::v2::core::api::endpoints::event_access_administration::remove_event_group;
    use futures::FutureExt;
    panel.apply(
        format!("Group {name} deleted"),
        move |store, event, revision| {
            async move { remove_event_group(store, &event, &group, revision).await }.boxed_local()
        },
    );
}

/// Add `member` to the roster, or remove them from it.
#[cfg(target_arch = "wasm32")]
fn send_member(panel: AccessPanel, group: String, member: Member, add: bool) {
    use crate::v2::core::api::endpoints::event_access_administration::{
        add_event_group_member, remove_event_group_member,
    };
    use futures::FutureExt;
    let label = if add {
        format!("{} added to the roster", member.username)
    } else {
        format!("{} removed from the roster", member.username)
    };
    panel.apply(label, move |store, event, revision| {
        async move {
            if add {
                add_event_group_member(store, &event, &group, &member.discord_id, revision).await
            } else {
                remove_event_group_member(store, &event, &group, &member.discord_id, revision).await
            }
        }
        .boxed_local()
    });
}
