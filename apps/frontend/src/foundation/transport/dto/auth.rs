//! Session, identity and account-linking payloads.
//!
//! **Role:** what the backend says about who the viewer is, whether their game account is
//! linked, the rotated token pair a refresh returns, and the member rows the pickers list.
//! **Position:** deserialised straight from the backend's JSON and handed to the session store and
//! the pages that render it; re-serialised unchanged by the round-trip tests and by the persisted
//! session blob.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** `MeResponse` is the authority on the current session's role; a page must never infer
//! one from anything else. Dates are carried as opaque strings so that re-serialising a profile
//! reproduces the bytes the backend sent. A refreshed pair always travels with the `Bearer` token
//! type and nothing else. A link code is short-lived and single-use on the backend. A member search
//! answers a paged envelope whose `total` counts every match, not only the rows on the page.
//! @contract arma-link.schema.json#/definitions/LinkStatus
//! @contract arma-link.schema.json#/definitions/LinkCode
//! @contract member-directory.schema.json#/definitions/MemberSearchPage

#[cfg(test)]
use super::common::Paginated;
use super::role::Role;
use serde::{Deserialize, Serialize};

/// The signed-in account, as the backend describes it.
///
/// Carries the identity, the linked game account if there is one, the role that gates every route,
/// the ban state, and the participation counters the profile renders.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub discord_id: String,
    pub username: String,
    pub discord_handle: String,
    pub avatar_url: String,
    /// Null when no game identity is linked. Kept as an explicit null in the persisted blob rather
    /// than omitted.
    #[serde(default)]
    pub arma_id: Option<String>,
    pub arma_character: String,
    pub role: Role,
    pub is_banned: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ban_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_at: Option<String>,
    pub total_deployments: i64,
    pub attendance_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// The rotated pair a refresh returns. On the wire it also carries `token_type`, which is always
/// `Bearer`: the read requires it and refuses any other value, and the write emits it, so the
/// type holds only the three values a session uses.
/// @contract session-token.schema.json#/definitions/SessionTokenPair
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "SessionTokenPairWire", into = "SessionTokenPairWire")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
}

/// The token scheme of a [`RefreshResponse`]; the contract fixes it to `Bearer`.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum BearerTokenType {
    Bearer,
}

/// The wire form of a [`RefreshResponse`]: its three values plus the constant token type.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionTokenPairWire {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    token_type: BearerTokenType,
}

#[cfg(any(target_arch = "wasm32", test))]
impl From<SessionTokenPairWire> for RefreshResponse {
    fn from(wire: SessionTokenPairWire) -> Self {
        Self {
            access_token: wire.access_token,
            refresh_token: wire.refresh_token,
            expires_at: wire.expires_at,
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl From<RefreshResponse> for SessionTokenPairWire {
    fn from(pair: RefreshResponse) -> Self {
        Self {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_at: pair.expires_at,
            token_type: BearerTokenType::Bearer,
        }
    }
}

/// The current session: the signed-in user, and whether their game account is linked.
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkCodeResponse {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// One member of the community, as the roster and pickers list them.
/// @contract member-directory.schema.json#/definitions/MemberSummary
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(test)]
pub type MemberSearchPage = Paginated<Member>;
