// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

///`QuotaUsageView`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct QuotaUsageView {
    pub guest: u64,
    pub legacy_unclassified: u64,
    pub member: u64,
    pub open: u64,
    pub total: u64,
}
