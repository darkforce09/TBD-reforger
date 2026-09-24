# Event groups section

The Groups tab of an [event](/documentation_v2/glossary.md#event)'s access sheet: the named sets of
accounts an access policy can admit, each a roster administrators maintain or the bot-verified
members of a partner Discord guild, with the forms that create, edit and delete them.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/event_manager/access/groups/
├── group_card.rs    one group's card: source, provenance, edit form, delete confirmation, roster
├── group_fields.rs  the name, source, guild id and role id fields both forms share
├── group_form.rs    `GroupForm`: the fields, their validation, the edit diff and the source wording
└── mod.rs           the module tree; `groups_section`, the create request and `account_names`
```

## How it works

`groups_section` renders the create form behind "New group" and one `group_card` per group of the
loaded access view. Every write goes through the access panel's change path (`AccessPanel::apply`
and `apply_then` in
`apps/website/frontend/src/v2/pages/administration/event_manager/access/state.rs`), which names the
access revision and adopts the view the [API](/documentation_v2/glossary.md#api) answers; the
section rebuilds from each new view, so a saved change shows at once. The create form clears and
closes only once the group exists, so a refused create leaves it filled for correction.

`GroupForm` holds text and is validated on save with the API's bounds: a trimmed name of 1 to 128
bytes, a partner guild id of 1 to 128 bytes, and at most 32 role ids of at most 128 bytes each,
typed separated by commas, spaces or new lines, with duplicates collapsed. `changes_from` sends
only the changed name and source, so an untouched edit sends nothing and toasts "No changes to
save". A source switch keeps the typed guild and role text, and `group_fields` rebuilds the partner
fields only when the kind changes, so typing never loses focus.

A managed roster card lists its members with who added each and when, a remove control and the
member search; a partner-guild card offers no roster, since its membership comes only from
bot-verified Discord observations. `account_names` maps every account id the access view and the
participant evidence name to a display name, and an unknown id shows as itself.

## Boundaries

- Depends on: `crate::v2::core::api::dto` (`EventGroupView`, `EventGroupSource`,
  `AuthorshipProvenance`, `EventGroupCreation`, `EventGroupChange`, `Member`, and the access view
  and participant evidence), `crate::v2::core::api::endpoints::event_access_administration` (the
  group and roster requests), `crate::v2::core::utils::utc_timestamp` (`utc_label`), the toast
  queue, and two modules of the parent folder,
  `apps/website/frontend/src/v2/pages/administration/event_manager/access/`: `state.rs`
  (`AccessPanel`) and `member_search.rs` (`MemberSearch`).
- Used by: in the same parent folder, `panel.rs`, which renders `groups_section` as the Groups
  tab, `participants_table.rs`, which words roster evidence with `authorship_line`, and
  `tests/access_drafts.rs` and `tests/access_evidence.rs`; `event_manager_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the four source files.
- Rules: the form refuses what the API would refuse
  (`a_group_form_refuses_what_the_backend_would`), an edit sends only what changed
  (`a_group_edit_sends_only_what_changed`, `an_untouched_group_edit_sends_nothing`), and role ids
  split on commas and whitespace without duplicates
  (`role_ids_split_on_commas_and_whitespace_without_duplicates`), all in the parent's
  `tests/access_drafts.rs`; a partner-guild group never shows roster controls.

## Related documentation

- [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
  — the access sheet's behaviour and what each access call means server-side.
