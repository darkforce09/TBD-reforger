//! The protected-action oracle: what a request carrying a generated session must receive.
//!
//! **Role:** Decides from an [`AuthorityCase`] alone whether the session is live, which site
//! rank it holds, and whether the membership it rests on is stale or override-extended; then
//! the status every rank gate must answer.
//!
//! **Position:** Pure test support for `tests/session_authority_properties.rs`; it never calls
//! production authorization code, so a production change that alters any of these rules
//! disagrees with it.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** the documented session and membership policy:
//! - a session is live only while its account is active (a ban or a deletion revokes every
//!   session of the account), it is neither logged out nor expired, the presented access
//!   credential is unexpired, and a development-issued session is served by a development
//!   configuration; rotating the refresh token keeps the session live;
//! - a development session holds its issued rank; any other session holds the guild-mapped
//!   rank (enlisted for a member with no mapped role) while the snapshot confirms membership
//!   verified in the past, less than 48 hours ago, or longer ago with an unexpired grace
//!   override; every other membership state is a guest;
//! - a snapshot is stale when it is absent, future-dated, at least 60 seconds old, or records
//!   a synchronization error; a development session reports no staleness;
//! - a refusal is 401 when the session is not live and 403 when its rank is below the gate.

use super::authority_cases::{
    AccountFate, AuthorityCase, FRESHNESS_SECONDS, GRACE_SECONDS, GraceOverride, GuildRoleMapping,
    MembershipStatus, Rank, ServingEnvironment, SessionFate, SessionProvenance, VerificationTime,
};
use axum::http::StatusCode;

/// The authority a generated session must carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedAuthority {
    /// The session authorizes requests at all.
    pub live: bool,
    /// The effective site rank of a live session.
    pub rank: Rank,
    /// The current profile reports a stale membership snapshot.
    pub membership_stale: bool,
    /// The current profile reports a grace override extending an expired snapshot.
    pub override_active: bool,
}

/// The rank, staleness and override standing the membership snapshot alone grants.
struct MembershipStanding {
    rank: Rank,
    stale: bool,
    override_active: bool,
}

impl ExpectedAuthority {
    /// The authority `case` must carry.
    pub fn of(case: &AuthorityCase) -> Self {
        let development_session = matches!(case.provenance, SessionProvenance::Development(_));
        let live = matches!(case.account_fate, AccountFate::Active)
            && matches!(
                case.session_fate,
                SessionFate::Untouched | SessionFate::RefreshRotated
            )
            && !(development_session && case.environment == ServingEnvironment::Production);
        let standing = membership_standing(case);
        let rank = match case.provenance {
            SessionProvenance::Development(rank) => rank,
            SessionProvenance::Ordinary => standing.rank,
        };
        Self {
            live,
            rank,
            membership_stale: !development_session && standing.stale,
            override_active: standing.override_active,
        }
    }

    /// The status a route gated on `gate` must answer for this session.
    pub fn status_for(&self, gate: Rank) -> StatusCode {
        if !self.live {
            StatusCode::UNAUTHORIZED
        } else if self.rank >= gate {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        }
    }
}

fn membership_standing(case: &AuthorityCase) -> MembershipStanding {
    let Some(snapshot) = case.snapshot else {
        return MembershipStanding {
            rank: Rank::Guest,
            stale: true,
            override_active: false,
        };
    };
    let mapped = match case.guild_role {
        GuildRoleMapping::Unmapped => Rank::Enlisted,
        GuildRoleMapping::Mapped(rank) => rank,
    };
    let age = match snapshot.verified {
        VerificationTime::SecondsAgo(age) => Some(age),
        VerificationTime::SecondsAhead(_) => None,
    };
    let override_unexpired = matches!(snapshot.grace_override, GraceOverride::ExpiresInSeconds(_));
    let (rank, override_active) = match (snapshot.status, age) {
        (MembershipStatus::Member, Some(age)) if age < GRACE_SECONDS => (mapped, false),
        (MembershipStatus::Member, Some(_)) if override_unexpired => (mapped, true),
        _ => (Rank::Guest, false),
    };
    MembershipStanding {
        rank,
        stale: snapshot.sync_error || age.is_none_or(|age| age >= FRESHNESS_SECONDS),
        override_active,
    }
}
