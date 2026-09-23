// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{
    EventAccessAdministrationContract, EventAccessPolicy, EventGroupView, QuotaUsageView,
    ReservationQuotas, SlotAccessPolicy, SquadAccessPolicy,
};

///The manager's view of one event's access configuration and usage.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventAccessAdministration {
    pub access_revision: u64,
    pub event_id: ::uuid::Uuid,
    pub event_policy: EventAccessPolicy,
    pub groups: ::std::vec::Vec<EventGroupView>,
    ///0 leaves the event uncapped.
    pub max_slots: u64,
    pub quota_usage: QuotaUsageView,
    pub reservation_quotas: ReservationQuotas,
    pub slot_policies: ::std::vec::Vec<SlotAccessPolicy>,
    pub squad_policies: ::std::vec::Vec<SquadAccessPolicy>,
}
impl ::std::convert::From<EventAccessAdministrationContract> for EventAccessAdministration {
    fn from(value: EventAccessAdministrationContract) -> Self {
        value.0
    }
}
