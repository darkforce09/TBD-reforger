// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{MissionReview, ReviewComment};

///GET /api/v1/missions/:id/reviews (the mission's author or an administrator): every review of the mission, newest first, and the review thread in order.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MissionReviewHistory {
    pub comments: ::std::vec::Vec<ReviewComment>,
    pub reviews: ::std::vec::Vec<MissionReview>,
}
