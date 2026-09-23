//! Pure authorization decisions from an explicitly verified Discord membership snapshot.

use chrono::{DateTime, Duration, Utc};

use crate::identity_and_access::models::user_account::UserRole;

/// Cached membership grants expire at this age unless an audited override extends them.
pub const DEFAULT_MEMBERSHIP_GRACE_PERIOD: Duration = Duration::hours(48);

/// A snapshot older than this age requires a visible synchronization warning.
pub const MEMBERSHIP_FRESHNESS_PERIOD: Duration = Duration::seconds(60);

/// Website authority determined from membership facts supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedMembershipPermissionDecision {
    /// Only a banned account has no authenticated role; other accounts retain Guest access.
    pub effective_role: Option<UserRole>,
    /// The snapshot is absent, future-dated, or older than the freshness period.
    pub stale: bool,
    /// An unexpired override is extending an otherwise expired membership snapshot.
    pub override_active: bool,
}

/// Resolve access without fetching Discord state or interpreting an empty role list as membership.
///
/// The caller supplies the last authoritative role and confirmed departure status. Snapshot age
/// and override expiry use `now`, making the decision deterministic. The grace-period and override
/// endpoints are exclusive. An override cannot bypass a ban, a confirmed departure, or an absent
/// or future-dated snapshot.
pub fn evaluate_cached_membership_permissions(
    now: DateTime<Utc>,
    last_verified_at: Option<DateTime<Utc>>,
    cached_role: UserRole,
    confirmed_nonmember: bool,
    override_until: Option<DateTime<Utc>>,
    banned: bool,
) -> CachedMembershipPermissionDecision {
    let age = last_verified_at
        .filter(|verified_at| *verified_at <= now)
        .map(|verified_at| now.signed_duration_since(verified_at));
    let stale = age.is_none_or(|age| age > MEMBERSHIP_FRESHNESS_PERIOD);
    let mut decision = CachedMembershipPermissionDecision {
        effective_role: Some(UserRole::Guest),
        stale,
        override_active: false,
    };

    if banned {
        decision.effective_role = None;
        return decision;
    }
    if confirmed_nonmember {
        return decision;
    }
    let Some(age) = age else {
        return decision;
    };

    if age < DEFAULT_MEMBERSHIP_GRACE_PERIOD {
        decision.effective_role = Some(cached_role);
    } else if override_until.is_some_and(|expires_at| now < expires_at) {
        decision.effective_role = Some(cached_role);
        decision.override_active = true;
    }
    decision
}

/// Keep recovery available to a previously verified administrator during a Discord outage.
///
/// This predicate authorizes only management of synchronization overrides. It must not replace
/// ordinary administrator authorization. A confirmed departure or ban removes this recovery grant;
/// an absent or future-dated snapshot cannot establish it. Snapshot age alone does not remove it.
pub fn can_manage_sync_override(
    now: DateTime<Utc>,
    last_verified_at: Option<DateTime<Utc>>,
    cached_role: UserRole,
    confirmed_nonmember: bool,
    banned: bool,
) -> bool {
    !banned
        && !confirmed_nonmember
        && cached_role == UserRole::Admin
        && last_verified_at.is_some_and(|verified_at| verified_at <= now)
}

#[cfg(test)]
#[path = "tests/cached_membership_permissions.rs"]
mod tests;
