# Event registration access

What the viewer may register for in an [event](/documentation_v2/glossary/a_to_f.md#event), and why: the
place outlook, the viewer's standing on each [mission](/documentation_v2/glossary/g_to_m.md#mission), the
seat restrictions of the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat), the wording of a refused
registration, the Places panel and the leader's waiting-list promotion.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/event_detail/registration_access/
├── mission_standing.rs    `MissionStanding`: the viewer's standing on a mission and what it offers
├── mod.rs                 the module tree
├── place_outlook.rs       `PlaceOutlook`: whether a place in the event can be had now, or why not
├── places_panel.rs        the Places panel: each pool, the places left and the viewer's own pool
├── refusal_notices.rs     `RegistrationRefusal`: every refusal code the API names, as a sentence
├── seat_eligibility.rs    whether a seat admits the viewer, and the policy that restricts it
├── tests/                 unit tests for the decisions and sentences, against captured dossiers
└── waitlist_promotion.rs  the "Promote from waiting list" control for a leader or administrator
```

## How it works

Everything here is a pure function of the `EventHub` the [API](/documentation_v2/glossary/a_to_f.md#api)
returned to this viewer, so a viewer admitted only by squad or
[slot](/documentation_v2/glossary/n_to_z.md#slot) policies sees only the missions and seats open to them;
the views only render the decisions, which are tested natively.

- `place_outlook` mirrors the API's choice of pool: one place per participant for the whole event,
  from the viewer's own pool (member or guest) first, then the open pool, never past the event-wide
  limit; otherwise the earliest opening of a pool with places left. An unknown pool state is closed.
- `MissionStanding::of` reads one mission's reservation state, held seat, waiting position,
  released signup, eligibility and pending Discord verification, and offers register (a place is
  free now), join the waiting list (a seat admits the viewer but no place or seat is left) or
  withdraw; `MissionStanding::unlisted`, for a mission the event does not list, withholds nothing.
- `seat_eligibility` opens a seat only for the exact value `eligible` and otherwise names the
  nearest policy set: the seat's, its squad's or the event's.
- `refusal_notices` words each refusal code (`ACCESS_POLICY`, `MEMBERSHIP_VERIFICATION_REQUIRED`,
  `QUOTA_NOT_OPEN`, `EVENT_FULL`, `MISSION_FULL`, `SEAT_NEEDED_BY_HOLDER`, `SEAT_TAKEN`,
  `SQUAD_HELD`, `NO_SEATS`, `REGISTRATION_CLOSED`, `ACCOUNT_UNAVAILABLE`,
  `DEPLOYMENT_REQUIREMENTS`), says which point at the waiting list, and shows the API's own
  sentence for any other; times show in the viewer's zone beside UTC, and no sentence names
  another participant.
- `waitlist_promotion` shows "Promote from waiting list" to the `leader`
  [role](/documentation_v2/glossary/n_to_z.md#role) and above and sends
  `POST /api/v1/event-missions/{emid}/waitlist/promote`; a toast reports the reply ("Seated 1
  participant from the waiting list.", "Nobody on the waiting list could be seated.") or words an
  `EVENT_FULL` refusal as "No place is free for anyone waiting: the operation or its pools are
  full.", and the mission card fetches again.

## Boundaries

- Depends on: `crate::v2::core` (`EventHub`, `OrbatSlot`, `ReservationQuotaAvailability`,
  `ApiRefusal`, the `event_registration` endpoint helpers, `has_min_role_authed`, `MaterialIcon`,
  `badge_class`, the date and UTC formatting) and the slotting selector's
  `can_register_reservation` in the parent folder.
- Used by: the parent folder's hub body, mission card, slotting selector, seat row and footer; the
  standalone slotting page in `apps/website/frontend/src/v2/pages/operations/orbat_selection/`,
  through the parent's re-export of `MissionStanding` and `standing_notices`; `event_hub_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: an unknown pool state reads as closed and an unknown seat standing as restricted, so the
  page never offers what the API would refuse (`an_unknown_pool_state_reads_as_closed`,
  `restricted_seats_are_closed_and_say_why` in `tests/registration_access.rs`); every documented
  refusal code is recognised (`every_documented_refusal_reason_is_recognised`).

## Related documentation

- [Event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  — the event dossier's behaviour and design.
