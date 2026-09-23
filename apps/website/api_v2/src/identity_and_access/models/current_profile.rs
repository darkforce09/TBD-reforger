//! Current account responses use effective session authority and cached membership status.

use super::User;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentProfileResponse {
    pub user: User,
    pub arma_linked: bool,
    pub membership_stale: bool,
    pub membership_override_active: bool,
    pub can_manage_membership_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedProfileResponse {
    pub user: User,
}
