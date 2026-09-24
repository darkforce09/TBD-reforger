# Event Operations Manager (`/admin/events`)

A month calendar of the unit's operations, the panel of whatever day is selected, the two frosted
forms that schedule a new operation or rewrite an existing one, and the access panel that decides
who may see and join an operation, from which pools, and why each participant is admitted.

## Architecture
- **`page.rs`**: route component — builds the screen's state, puts it behind the administrator
  gate, and composes the calendar, the four dialogs and the access sheet in the order the stacking
  depends on.
- **`state.rs`**: the one copyable handle every panel reads — the calendar's position, both forms'
  fields, the in-flight flags, the three fetches, the day grouping derived from them, and the
  access panel's own handle.
- **`event_table.rs`**: the page heading, the month grid with one cell per day, the selected day's
  operations, and the controls that open the edit form, open the access panel and arm the delete.
- **`schedule_dialog.rs`**: the Schedule Operation form and the two-step publish behind it.
- **`edit_dialog.rs`**: the Edit Operation form and the save that sends only what changed.
- **`mission_picker.rs`**: the two places missions are attached — the schedule form's staging list,
  and the edit form's live roster with its detach control.
- **`confirm_dialogs.rs`**: the delete confirmation and the detach confirmation, with the two
  requests they guard.
- **`dates.rs`**: the date arithmetic — local day keys, the values the date and time fields want,
  and the instant a wall clock reads back as.
- **`lifecycle.rs`**: the six operation states, the moves the server will accept between them, the
  badge each is shown with, and the delete confirmation's copy.
- **`access/`**: the access panel, a side sheet over the calendar with four sections —
  - `state.rs`: the panel's handle — the operation it is open on, its three reads (the access view
    of `GET /events/:id/access`, the participant evidence of `GET /events/:id/access/participants`,
    and the missions with their orders of battle) and the one change path every change goes
    through, which names the access revision the view was read at, adopts the view the change
    answers with, and on `409 ACCESS_REVISION_CONFLICT` reloads the view and tells the operator the
    change was not applied;
  - `change_report.rs`: the notice after a change — the reservations it released and the waiting
    participants it promoted, named by participant and mission — and the wording of a refusal;
  - `panel.rs`: the sheet — heading with the access revision, the change notice, the four tabs;
  - **Policies** — `policy_lists.rs` (the operation's policy, and each mission's squads and slots
    stating where their policy comes from, with "Give it its own policy", "Edit own policy" and
    "Inherit again"), `policy_editor.rs` (grants as alternatives, the conditions inside a grant all
    required; kinds: any signed-in account, verified TBD member, Discord role, event group member,
    named account), `policy_draft.rs` (the editor's plain rows, their validation and summaries) and
    `policy_inheritance.rs` (slot, then squad, then operation). Removing a squad's or slot's own
    policy (`DELETE`) makes it inherit; saving a policy with no grants admits nobody — the editor
    says so before it is saved;
  - **Groups** — `groups/` (managed rosters, with the members who were added, by whom and when, a
    remove control and a member search on `GET /members?q=`; partner-guild groups with the guild id
    and the roles a member must hold, verified by the bot; each group's provenance; creation,
    rename, source change and deletion, a group some policy still names being refused), and
    `member_search.rs`;
  - **Places** — `quota_editor.rs`: the member, guest and open pools, each capped or uncapped, with
    its opening time in UTC, against the places held and the operation-wide limit;
  - **Participants** — `participants_table.rs`: each participant's availability, TBD membership,
    place and pool, every reservation with the policy that decides it, the grants that admit it and
    whether current and last-verified facts admit it, and the Discord guild observations and roster
    entries behind those facts;
  - `waitlist_promotion.rs`: each mission's "Promote from waiting list" control, which names the
    participants it seated.
- **`tests/event_manager.rs`**: the edit form's attach and clear wiring, and the delete copy held
  against what the endpoint actually does.
- **`access/tests/`**: the policy drafts, group forms and pool forms read back into what the backend
  takes, and the inheritance, evidence, change-report and refusal wording, held against the
  captured access view, participant evidence and order of battle.

## Not present in the legacy page
- **A Discord announcement control**: neither form has one. Publishing an operation sends nothing
  to chat from this screen.
- **Modpack configuration**: an operation carries no modpack. The forms cover time, name, briefing,
  banner, slot ceiling, attached missions, lifecycle state and registration, and nothing else.
- **Publish / cancel / archive actions on the table**: the day panel offers Edit, Access and Delete.
  The lifecycle state is set in the edit form's picker, which offers only the moves the server
  accepts.

## Related documentation

- [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
