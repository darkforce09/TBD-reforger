// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ReservationQuotas;

///`ReservationQuotaChange`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaChange {
    pub expected_access_revision: i64,
    pub reservation_quotas: ReservationQuotas,
}
