// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/waitlist-promotion-response.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/event-missions/:emid/waitlist/promote (leader or administrator). Promotes the earliest eligible waiters in queue order into actual seats and quota places; answers 409 with details.code EVENT_FULL when no place can be given, leaving every waiter waiting.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WaitlistPromotionResponse {
    pub promoted: ::std::vec::Vec<WaitlistPromotionResponsePromotedItem>,
}
///`WaitlistPromotionResponsePromotedItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WaitlistPromotionResponsePromotedItem {
    pub discord_id: ::std::string::String,
    pub registration_id: ::uuid::Uuid,
    pub slot_id: ::uuid::Uuid,
}
