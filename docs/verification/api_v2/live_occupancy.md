# Live slot occupancy and deployment authorization

This note records the implemented design of live occupancy. It describes the code; acceptance
evidence is the command output recorded in progress_checkpoint.md.

## Model

`live_slot_occupancies` (migration 0048) records one player life in one ORBAT slot during one
runtime session: the Arma identity, the account it belonged to, the runtime's `player_life_id`,
what authorized it (`reservation` or `open_slot_policy`), and when and why it ended. Partial unique
indexes allow at most one open life per slot and one open life per player per session;
`(runtime_session_id, player_life_id)` is unique.

## Deployment decision

`POST /game-runtime/sessions/{sessionId}/deployments` (`operations/services/live_slot_occupancy.rs`)
allows a life when the identity is linked, the account is available, and either the player's own
active reservation is for the slot, or the slot is unreserved and its effective policy admits the
player under current membership authority — and the slot has no other open life and the player no
other open life in the session. Otherwise it answers 200 with `decision: denied` and one reason:
`IDENTITY_NOT_LINKED`, `ACCOUNT_UNAVAILABLE`, `SLOT_RESERVED`, `RESERVED_ANOTHER_SLOT`,
`ACCESS_POLICY`, `MEMBERSHIP_VERIFICATION_REQUIRED`, `LIVE_SLOT_OCCUPIED` or
`PLAYER_ALREADY_DEPLOYED`. An allowed decision is recorded under the life id, so a retry after a
lost acknowledgement returns it unchanged even when the reservation changed in between; reusing a
life id for another slot is refused (409).

Occupancy is independent of reservations. Releasing a reservation, for any reason, never ends a
life; it only makes the next deployment of that player be refused, and a replacement cannot spawn
into the still-occupied slot. `POST .../deployments/{occupancyId}/end` ends exactly one life, so a
delayed or repeated end cannot clear a newer occupant. Superseding, expiring or ending a session,
or revoking its credential, ends the session's open lives.

## Lock order

Event (share) → attachment (share) → identity → account → slot → runtime session (share).
Reservation writers take the event, attachment and account locks exclusively in the same order;
session writers take only session and occupancy rows. An allowed decision and its occupancy row
commit together.

## Roster

`GET /game-runtime/events/{id}/roster` is wire version 2: each assignment carries `armaId`,
`slotUid`, `orbatSlotId` and `eventMissionId`, and `slots` lists every compiled slot with the ids a
deployment request names.

## Tests

`tests/live_slot_occupancy.rs` (`live_occupancy_*`) and the roster suites
`tests/events_roster_and_members.rs` and `tests/roster_whitespace_arma_id.rs`.
