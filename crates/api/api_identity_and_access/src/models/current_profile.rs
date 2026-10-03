//! Current account responses use effective session authority and cached membership status.
//!
//! @contract current-profile.schema.json#
//! @contract profile-update.schema.json#/definitions/UpdatedProfile

use super::User;
use serde::{Deserialize, Serialize};

/// The `GET /api/v1/me` answer: the account with its link and membership status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentProfileResponse {
    /// The caller's account row, its role the effective session authority.
    pub user: User,
    /// Whether an Arma identity is linked to the account.
    pub arma_linked: bool,
    /// Whether the cached Discord membership snapshot is past its freshness window.
    pub membership_stale: bool,
    /// Whether an audited membership grace extension is in force.
    pub membership_override_active: bool,
    /// Whether the caller may extend another member's membership grace.
    pub can_manage_membership_override: bool,
}

/// The `PATCH /api/v1/me` answer: the updated account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedProfileResponse {
    /// The caller's account row after the update.
    pub user: User,
}
