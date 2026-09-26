# Event access administration

The storage behind the access settings an administrator manages for one
[event](/documentation_v2/glossary/a_to_f.md#event): the event, squad and
[slot](/documentation_v2/glossary/n_to_z.md#slot) access policies, the event's groups and their rosters,
and the member, guest and open reservation pools, with the evidence that explains why each
participant is or is not admitted.

## Contents

```text
apps/website/api_v2/src/operations/services/access_administration/
├── mod.rs                      the module tree
├── participant_explanation.rs  why each participant is or is not admitted, and the facts behind it
└── persistence.rs              store policies, groups and pools; read the manager's access view
```

## How it works

The callers hold the administrator event scope. Every change names the access revision its form
was loaded at: `advance_access_revision` answers 409 `ACCESS_REVISION_CONFLICT` with the current
revision when they differ, and otherwise advances it, so a stale form never overwrites a newer
one. A policy may name only this event's groups and the guilds that apply to it; storing `None`
for a squad or slot policy removes it, so the seats inherit the broader policy again. A group that
any policy still names cannot be removed. `explain_participants` reads every account with a
reservation, a waiting entry, a seat or an allocation in one snapshot and lists, for each, the
effective policy, the grants that hold and the provenance of each membership fact they use.

## Boundaries

- Depends on: `operations::services::event_access` for the access context, the grant evaluation
  and the effective slot policies; `operations::services::event_reservations` for pool usage and
  the stored quotas; `operations::models`; `core` for errors.
- Used by: the handlers `event_access_administration.rs` and `event_group_administration.rs` in
  `apps/website/api_v2/src/operations/handlers/`.
- Rules: every stored change first advances the access revision; a group referenced by a policy
  answers 409 when deleted (`event_groups_referenced_group_deletion_conflicts` in
  `apps/website/api_v2/tests/event_eligibility_policies.rs`); a policy that names another event's
  group or an unknown guild is refused
  (`event_access_policy_references_require_live_groups_and_guilds_from_the_same_event` in
  `apps/website/api_v2/tests/event_access_context.rs`).

## Related documentation

- [Event eligibility and allocation](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — policies, groups, quotas and how changes re-evaluate reservations.
