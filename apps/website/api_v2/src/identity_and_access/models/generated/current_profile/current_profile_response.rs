// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/current-profile.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::UserAccount;

///GET /api/v1/me. Backend identity models define the snake_case contract.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CurrentProfileResponse {
    pub arma_linked: bool,
    pub can_manage_membership_override: bool,
    pub membership_override_active: bool,
    pub membership_stale: bool,
    pub user: UserAccount,
}
