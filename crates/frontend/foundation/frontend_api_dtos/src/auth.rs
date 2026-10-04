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
use super::identifiers::{ArmaPlayerId, DiscordUserId};
use super::role::Role;
use serde::{Deserialize, Serialize};

/// The signed-in account, as the backend describes it.
///
/// Carries the identity, the linked game account if there is one, the role that gates every route,
/// the ban state, and the participation counters the profile renders.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// The Discord user id, the account's key.
    pub discord_id: DiscordUserId,
    /// The Discord username.
    pub username: String,
    /// The Discord handle.
    pub discord_handle: String,
    /// The Discord avatar URL; empty when the account has none.
    pub avatar_url: String,
    /// Null when no game identity is linked. Kept as an explicit null in the persisted blob rather
    /// than omitted.
    #[serde(default)]
    pub arma_id: Option<ArmaPlayerId>,
    /// The linked Arma character name; empty until linked.
    pub arma_character: String,
    /// The account's role on the wire.
    pub role: Role,
    /// Whether the account is banned.
    pub is_banned: bool,
    /// Why the account is banned; omitted when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ban_reason: String,
    /// The Discord id of the administrator who banned the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_by: Option<String>,
    /// When the account was banned, as an RFC 3339 instant; absent while not banned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_at: Option<String>,
    /// The number of finished deployments attributed to the account.
    pub total_deployments: i64,
    /// The share of scheduled operations the account attended, in percent.
    pub attendance_rate: f64,
    /// When the account last signed in, as an RFC 3339 instant; absent before the first sign-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    /// When the account was created.
    pub created_at: String,
    /// When the account row last changed.
    pub updated_at: String,
}

/// The rotated pair a refresh returns. On the wire it also carries `token_type`, which is always
/// `Bearer`: the read requires it and refuses any other value, and the write emits it, so the
/// type holds only the three values a session uses.
/// @contract session-token.schema.json#/definitions/SessionTokenPair
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "SessionTokenPairWire", into = "SessionTokenPairWire")]
pub struct RefreshResponse {
    /// The new short-lived access token.
    pub access_token: String,
    /// The rotated refresh token that replaces the one sent.
    pub refresh_token: String,
    /// When the access token expires, as an RFC 3339 instant.
    pub expires_at: String,
}

/// The token scheme of a [`RefreshResponse`]; the contract fixes it to `Bearer`.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum BearerTokenType {
    Bearer,
}

/// The wire form of a [`RefreshResponse`]: its three values plus the constant token type.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionTokenPairWire {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    token_type: BearerTokenType,
}

impl From<SessionTokenPairWire> for RefreshResponse {
    fn from(wire: SessionTokenPairWire) -> Self {
        Self {
            access_token: wire.access_token,
            refresh_token: wire.refresh_token,
            expires_at: wire.expires_at,
        }
    }
}

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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    /// The caller's account row, its role the effective session authority.
    pub user: User,
    /// Whether an Arma identity is linked to the account.
    pub arma_linked: bool,
    /// Whether the cached Discord membership snapshot is past its freshness window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership_stale: Option<bool>,
    /// Whether an audited membership grace extension is in force.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership_override_active: Option<bool>,
    /// Whether the caller may extend another member's membership grace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_manage_membership_override: Option<bool>,
}

/// Whether the viewer has linked a game account, and what is known about it.
/// @contract arma-link.schema.json#/definitions/LinkStatus
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStatus {
    /// Whether an Arma identity is linked to the account.
    pub linked: bool,
    /// The linked Arma identity; absent until linked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_id: Option<ArmaPlayerId>,
    /// The linked character name; absent until linked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_character: Option<String>,
    /// Whether a link code is waiting to be entered in game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_code: Option<bool>,
}

/// A freshly minted account-linking code, and when it stops being accepted.
/// @contract arma-link.schema.json#/definitions/LinkCode
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkCodeResponse {
    /// The code to enter in game to link the account.
    pub code: String,
    /// When the code stops being accepted, as an RFC 3339 instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// One member of the community, as the roster and pickers list them.
/// @contract member-directory.schema.json#/definitions/MemberSummary
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// The Discord user id, the account's key.
    pub discord_id: DiscordUserId,
    /// The Discord username.
    pub username: String,
    /// The Discord avatar URL; empty when the account has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// One page of a member search: members who are not banned, by username, with the paging the
/// pickers read.
/// @contract member-directory.schema.json#/definitions/MemberSearchPage
#[cfg(test)]
pub type MemberSearchPage = Paginated<Member>;
