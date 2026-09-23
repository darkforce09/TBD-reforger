// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ReservationQuotaPool;

///Members draw from member, others from guest; either overflows to open once open opens.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotas {
    pub guest: ReservationQuotaPool,
    pub member: ReservationQuotaPool,
    pub open: ReservationQuotaPool,
}
