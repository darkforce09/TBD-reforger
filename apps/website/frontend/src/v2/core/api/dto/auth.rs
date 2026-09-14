//! Session, identity and account-linking payloads.
//!
//! **Role:** what the backend says about who the viewer is, whether their game account is
//! linked, and the rows the personnel screens list.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** `MeResponse` is the authority on the current session's role; a page must never infer
//! one from anything else. A link code is short-lived and single-use on the backend.

use crate::v2::core::auth::User;
use serde::{Deserialize, Serialize};

/// The current session: the signed-in user, and whether their game account is linked.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: User,
    pub arma_linked: bool,
}

/// Whether the viewer has linked a game account, and what is known about it.
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
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkCodeResponse {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// One member of the community, as the roster and pickers list them.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub discord_id: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// One row of the personnel roster, as the administration screens list it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminUserRow {
    pub discord_id: String,
    pub username: String,
    pub discord_handle: String,
    #[serde(default)]
    pub arma_id: Option<String>,
    pub arma_character: String,
    pub role: crate::shell::nav_config::Role,
    pub is_banned: bool,
    pub warnings: i64,
    pub total_deployments: i64,
}
