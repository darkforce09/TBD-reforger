//! The name, kind, guild and role fields shared by the create form and a group's edit form.
//!
//! **Role:** renders the group form's fields bound to one form signal: the name, the choice between
//! a managed roster and a partner guild, and — for a partner guild — the guild id and the roles a
//! member must hold.
//! **Position:** inside the Groups section's create form and each group card's edit form.
//! **Signals & state:** reads and writes the caller's form signal; owns nothing.
//! **Invariants:** the partner fields are shown only for a partner guild, and a kind switch keeps
//! what was typed in them, so switching back loses nothing. Only a change of kind rebuilds them, so
//! typing never loses the field's focus. The fields hold text; the form is validated when it is
//! saved.

use super::group_form::{GroupForm, GroupKind};
use leptos::prelude::*;

/// Shared styling for the form's fields.
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// The fields of one group form.
pub(super) fn group_fields(form: RwSignal<GroupForm>) -> impl IntoView {
    // A memo, so typing into a field — which rewrites the whole form signal — never rebuilds the
    // partner fields being typed into; only a change of kind does.
    let kind = Memo::new(move |_| form.with(|f| f.kind));
    let is_roster = move || kind.get() == GroupKind::ManagedRoster;
    let is_partner = move || kind.get() == GroupKind::PartnerGuild;
    view! {
        <div class="grid gap-2 md:grid-cols-2">
            <label class="flex flex-col gap-1 text-xs text-on-surface-variant">
                "Group name"
                <input
                    prop:value=move || form.with(|f| f.name.clone())
                    on:input=move |ev| form.update(|f| f.name = event_target_value(&ev))
                    placeholder="Allied reconnaissance"
                    class=FIELD
                />
            </label>
            <label class="flex flex-col gap-1 text-xs text-on-surface-variant">
                "Members come from"
                <select
                    on:change=move |ev| {
                        if let Some(kind) = GroupKind::from_wire(&event_target_value(&ev)) {
                            form.update(|f| f.kind = kind);
                        }
                    }
                    class=FIELD
                >
                    <option value=GroupKind::ManagedRoster.wire() prop:selected=is_roster>
                        "A roster administrators maintain"
                    </option>
                    <option value=GroupKind::PartnerGuild.wire() prop:selected=is_partner>
                        "A partner Discord guild, verified by the bot"
                    </option>
                </select>
            </label>
            {move || {
                is_partner()
                    .then(|| {
                        view! {
                            <label class="flex flex-col gap-1 text-xs text-on-surface-variant">
                                "Partner guild id"
                                <input
                                    prop:value=move || form.with(|f| f.guild_id.clone())
                                    on:input=move |ev| form.update(|f| f.guild_id = event_target_value(&ev))
                                    placeholder="100000000000000777"
                                    class=FIELD
                                />
                            </label>
                            <label class="flex flex-col gap-1 text-xs text-on-surface-variant">
                                "Required role ids — every one must be held; none means guild membership alone"
                                <input
                                    prop:value=move || form.with(|f| f.role_ids.clone())
                                    on:input=move |ev| form.update(|f| f.role_ids = event_target_value(&ev))
                                    placeholder="200000000000000888, 200000000000000889"
                                    class=FIELD
                                />
                            </label>
                        }
                    })
            }}
        </div>
    }
}
