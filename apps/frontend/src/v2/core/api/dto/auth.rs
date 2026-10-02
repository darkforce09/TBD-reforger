//! Session, identity and account-linking payloads.
//!
//! **Role:** what the backend says about who the viewer is, whether their game account is
//! linked, and the member rows the pickers list.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** `MeResponse` is the authority on the current session's role; a page must never infer
//! one from anything else. A link code is short-lived and single-use on the backend. A member search
//! answers a paged envelope whose `total` counts every match, not only the rows on the page.
//! @contract arma-link.schema.json#/definitions/LinkStatus
//! @contract arma-link.schema.json#/definitions/LinkCode
//! @contract member-directory.schema.json#/definitions/MemberSearchPage

use super::common::Paginated;
use crate::v2::core::auth::User;
use serde::{Deserialize, Serialize};

/// The current session: the signed-in user, and whether their game account is linked.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: User,
    pub arma_linked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership_stale: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership_override_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_manage_membership_override: Option<bool>,
}

/// Whether the viewer has linked a game account, and what is known about it.
/// @contract arma-link.schema.json#/definitions/LinkStatus
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStatus {
    pub linked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_code: Option<bool>,
}

/// A freshly minted account-linking code, and when it stops being accepted.
/// @contract arma-link.schema.json#/definitions/LinkCode
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkCodeResponse {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// One member of the community, as the roster and pickers list them.
/// @contract member-directory.schema.json#/definitions/MemberSummary
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub discord_id: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// One page of a member search: members who are not banned, by username, with the paging the
/// pickers read.
/// @contract member-directory.schema.json#/definitions/MemberSearchPage
#[allow(dead_code)]
pub type MemberSearchPage = Paginated<Member>;
