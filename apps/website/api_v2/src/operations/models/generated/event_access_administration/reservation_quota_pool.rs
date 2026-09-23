// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

///`ReservationQuotaPool`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaPool {
    pub opens_at: ::chrono::DateTime<::chrono::offset::Utc>,
    ///null leaves the pool uncapped; 0 closes it.
    pub seats: ::std::option::Option<u32>,
}
