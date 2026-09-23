// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::EventAccessAdministration;

///GET /api/v1/events/:id/access (administrator). Every change names expected_access_revision; a stale revision answers 409 ACCESS_REVISION_CONFLICT. Changes answer AccessChangeOutcome: PUT /events/:id/access-policy, PUT and DELETE (?expected_access_revision=) /event-missions/:emid/squads/:faction/:squad/access-policy and /event-missions/:emid/slots/:slotId/access-policy, PUT /events/:id/reservation-quotas, POST /events/:id/groups (201), PATCH and DELETE (?expected_access_revision=) /events/:id/groups/:groupId, PUT and DELETE (?expected_access_revision=) /events/:id/groups/:groupId/members/:discordId.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct EventAccessAdministrationContract(pub EventAccessAdministration);
impl ::std::ops::Deref for EventAccessAdministrationContract {
    type Target = EventAccessAdministration;
    fn deref(&self) -> &EventAccessAdministration {
        &self.0
    }
}
impl ::std::convert::From<EventAccessAdministration> for EventAccessAdministrationContract {
    fn from(value: EventAccessAdministration) -> Self {
        Self(value)
    }
}
