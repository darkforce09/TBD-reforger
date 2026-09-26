# Event access sheet

The side sheet the [event manager](/documentation_v2/glossary/a_to_f.md#event-manager) opens on one
[event](/documentation_v2/glossary/a_to_f.md#event), which the screen calls an operation: who may see and
join it, which groups a policy can admit, how many places each pool holds, and why each participant
is admitted, with the waiting-list promotion of each
[mission](/documentation_v2/glossary/g_to_m.md#mission).

## Contents

```text
apps/website/frontend/src/v2/pages/administration/event_manager/access/
├── change_report.rs        what a change released and promoted, and the wording of a refusal
├── groups/                 the Groups tab: event groups, their rosters and their forms
├── member_search.rs        `MemberSearch`: the member directory typeahead
├── mod.rs                  the module tree; re-exports `AccessPanel` and `access_sheet`
├── panel.rs                the sheet: heading, change notice, the four tabs and the tab on screen
├── participants_table.rs   the Participants tab: each participant's place, reservations and evidence
├── policy_draft.rs         a policy as the editor holds it, its validation and one-line summaries
├── policy_editor.rs        the grants-and-conditions editor every policy is edited in
├── policy_inheritance.rs   where a squad's or slot's policy comes from: its own, the squad's, the event's
├── policy_lists.rs         the Policies tab: the event's policy, then each mission's squads and slots
├── quota_editor.rs         the Places tab: the member, guest and open pools and their usage
├── state.rs                `AccessPanel`: the event the sheet is on, its reads and the change path
├── tests/                  unit tests for the drafts, the forms, the evidence lines and the wording
└── waitlist_promotion.rs   each mission's "Promote from waiting list" control
```

## How it works

The day panel's "Access, Groups & Places" calls `AccessPanel::open_on`, which opens the sheet on
the Policies tab and starts three reads: the access view, the participant evidence, and the
missions with their [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat), read as the event's hub and then
each event mission's squads. A read writes its signal only while the sheet is still open on the
event it asked about, so a late answer never shows against another event. Nothing is editable
until the access view has loaded, because every change names the revision it was read at; a
failed read shows its reason, never an empty configuration.

Every write except the promotion goes through one change path, `AccessPanel::apply` (or
`apply_then`, for a form that clears itself only once the change lands):

```text
click ──► busy? or no view yet? ──► ignore
      └─► send with expected_access_revision
            ├─ accepted: AccessChangeOutcome ─► adopt its view (no second read)
            │     name released and promoted registrations from the participants read before
            │     notice + toast; read the participant evidence again
            └─ refused ─► notice + toast with the API's sentence
                  └─ 409 ACCESS_REVISION_CONFLICT ─► read the view and the evidence again
```

| Tab | Files | What it changes |
|---|---|---|
| Policies | `policy_lists.rs`, `policy_editor.rs`, `policy_draft.rs`, `policy_inheritance.rs`, `waitlist_promotion.rs` | the event's policy; a squad's or a [slot](/documentation_v2/glossary/n_to_z.md#slot)'s own policy, set or removed; a mission's waiting list |
| Groups | `groups/`, `member_search.rs` | event groups and managed-roster members |
| Places | `quota_editor.rs` | the three reservation pools, replaced together |
| Participants | `participants_table.rs` | nothing: it reads the evidence |

The invariants that span the tabs:

- Grants are alternatives and every condition inside a grant must hold. A policy with no grants
  admits nobody, while removing a squad's or slot's own policy makes it inherit; the sheet keeps
  the two visibly apart. A slot's own policy wins, then its squad's, then the event's.
- The open policy editor shuts whenever the view's revision moves, so a draft prepared against an
  older view is never saved over a newer one; a new own policy starts from the inherited one.
- The draft enforces the [API](/documentation_v2/glossary/a_to_f.md#api)'s bounds before sending: at most
  32 grants (`policy_draft::MAX_GRANTS`) of 1 to 16 conditions (`MAX_CONDITIONS`), identifiers of
  1 to 128 bytes without control characters.
- An uncapped pool is sent as an explicit null limit and zero closes a pool; an opening time left
  untouched is sent back exactly as read.
- Grant numbers show one-based though the wire counts from zero, and a reservation that current
  facts no longer admit still shows as standing, since only the last verified facts release one.
- A promotion is not an access change: it names no revision, leaves the view alone and shares the
  `busy` flag, so it never runs beside a change.

## Public surface

- `AccessPanel`: `new`, which the event manager's `state.rs` calls to build the shut sheet inside
  the route, and `open_on`, which its `event_table.rs` calls from the day panel.
- `access_sheet`: the sheet view the event manager's `page.rs` renders over the calendar.

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `ApiRefusal`, `api_error_message`; the DTOs
  `EventAccessAdministration`, `AccessChangeOutcome`, `ParticipantAccessExplanation`, the policy,
  group and quota change bodies, `EventHub`, `OrbatSquad`, `WaitlistPromotion` and `Member`; the
  endpoint modules `event_access_administration` and `event_registration`), `crate::v2::core::auth`
  (`AuthStore`), `crate::v2::core::ui` (`Sheet`, `SearchBox`, `MaterialIcon`, the toast queue) and
  `crate::v2::core::utils` (`utc_timestamp`, `safe_avatar_url`); over HTTP, the access, group,
  quota, waitlist, member directory and ORBAT routes of the
  [operations](/documentation_v2/glossary/n_to_z.md#operations) domain.
- Used by: `state.rs`, `event_table.rs` and `page.rs` of
  `apps/website/frontend/src/v2/pages/administration/event_manager/`; `event_manager_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins these sources for the
  event manager's tests.
- Rules: every change names the access revision, and a stale one reloads the view instead of
  overwriting (`a_stale_revision_is_told_apart_from_other_conflicts`); an empty draft admits
  nobody (`an_empty_draft_is_a_policy_that_admits_nobody`) and an own policy without grants says
  so (`an_own_policy_without_grants_reads_as_admitting_nobody`); inheritance resolves slot, then
  squad, then event (`captured_overrides_resolve_slot_then_squad_then_operation`); untouched pools
  go back exactly (`untouched_pools_are_sent_back_exactly`); the member search asks only for a
  typed query (`the_member_search_asks_only_for_a_typed_query`). The tests read the captured
  fixtures in `apps/website/frontend/tests/fixtures/api/`.

## Related documentation

- [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
  — the access sheet's behaviour, what each access call means server-side, and its design.
- [Event eligibility and allocation evidence](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — how the API evaluates policies, pools and reservations.
