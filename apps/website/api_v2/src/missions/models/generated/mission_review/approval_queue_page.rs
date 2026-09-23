// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ApprovalQueueRow;

///GET /api/v1/approvals (administrator): missions awaiting review, oldest submission first.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ApprovalQueuePage {
    pub data: ::std::vec::Vec<ApprovalQueueRow>,
    pub limit: ::std::num::NonZeroU64,
    pub offset: u64,
    pub total: u64,
}
