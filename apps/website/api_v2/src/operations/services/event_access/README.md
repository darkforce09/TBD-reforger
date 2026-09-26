# Event access evaluation

Who may see and who may take part in an [event](/documentation_v2/glossary/a_to_f.md#event): the pure
evaluation of an access policy against one account's facts, the effective policy of each
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) [slot](/documentation_v2/glossary/n_to_z.md#slot), the
membership facts loaded under both evidence standards, and what one viewer may see of each event.

## Contents

```text
apps/website/api_v2/src/operations/services/event_access/
├── context.rs           `EventAccessContext`: one event's policies, groups and guilds
├── evaluation.rs        pure evaluation: effective policy, grants, and the gates no grant overrides
├── mod.rs               the module tree
├── slot_eligibility.rs  the effective policy of a concrete seat, slot then squad then event
├── subject_loading.rs   membership facts for many accounts, under current and last verified evidence
├── tests/               unit tests for the policy evaluation
└── visibility.rs        which events, attachments and seats one viewer may see
```

## How it works

A policy is a list of grants and each grant a list of conditions (`authenticated`, `tbd_member`,
`discord_role`, `event_group`, `named_account`): the grants are alternatives and the conditions of
one grant must all hold. A slot policy replaces its squad's, which replaces the event's; a missing
policy inherits, and an empty grant list admits nobody. `evaluate_access` checks the mandatory gates
apart from the policy, and no grant overrides them: a valid session, an available account, open
registration, the pool's opening time, [deployment](/documentation_v2/glossary/a_to_f.md#deployment)
conditions and capacity.

`subject_loading.rs` builds an account's facts under two standards. Current authority accepts
membership snapshots that are fresh, inside the 48-hour grace period, or under an audited
override; it decides every new action. Last verified facts keep the latest verified member
observation whatever its age; they decide whether a loss of eligibility is confirmed, so stale or
unreachable Discord data never evicts a reservation. A policy denial that pending evidence could
still lift becomes `MembershipVerificationRequired`, and pending evidence never grants.

`visibility.rs` answers `Full` when the event policy admits the viewer, `Partial` with the admitted
seats when only squad or slot policies do, and `Hidden` otherwise; administrators see every event.
Registration status, capacity and opening times never hide an event.

## Boundaries

- Depends on: `operations::models` (the policies, groups and pool kinds); `identity_and_access`
  for `UserRole` and `evaluate_cached_membership_permissions`, which applies the grace period and
  the overrides; `core` for errors; the `discord_membership_snapshots`,
  `discord_membership_grace_overrides`, `user_discord_roles`, `event_groups` and
  `event_group_roster` tables.
- Used by: the [operations](/documentation_v2/glossary/n_to_z.md#operations) handlers `event_listing.rs`,
  `event_hub.rs`, `orbat_view.rs` and `event_group_administration.rs`;
  `operations::services::event_reservations`, `operations::services::access_administration` and
  `operations::services::live_slot_occupancy`; the models `event_access_administration.rs` and
  `event_viewer_access.rs` in `apps/website/api_v2/src/operations/models/`; the test
  `apps/website/api_v2/tests/event_access_context.rs`.
- Rules: exactly one policy decides a seat, slot then squad then event
  (`slot_then_squad_then_event_selects_exactly_one_policy`); every mandatory gate blocks every
  grant (`every_mandatory_gate_blocks_every_satisfied_grant`); a malformed effective policy fails
  closed and never falls back to an open ancestor
  (`invalid_effective_policy_fails_closed_and_cannot_fall_back_to_open_ancestor`), all in
  `tests/evaluation.rs`.

## Related documentation

- [Event eligibility and allocation](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — access policies, evidence standards and visibility.
