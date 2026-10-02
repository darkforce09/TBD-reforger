use super::{
    DEFAULT_MEMBERSHIP_GRACE_PERIOD, MEMBERSHIP_FRESHNESS_PERIOD, can_manage_sync_override,
    evaluate_cached_membership_permissions,
};
use crate::identity_and_access::models::user_account::UserRole;
use chrono::{DateTime, Duration, Utc};
use proptest::prelude::*;

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

#[test]
fn discord_resilience_snapshot_age_boundaries() {
    let now = now();
    let nanosecond = Duration::nanoseconds(1);
    let cases = [
        (Duration::zero(), UserRole::Admin, false),
        (MEMBERSHIP_FRESHNESS_PERIOD, UserRole::Admin, false),
        (
            MEMBERSHIP_FRESHNESS_PERIOD + nanosecond,
            UserRole::Admin,
            true,
        ),
        (
            DEFAULT_MEMBERSHIP_GRACE_PERIOD - nanosecond,
            UserRole::Admin,
            true,
        ),
        (DEFAULT_MEMBERSHIP_GRACE_PERIOD, UserRole::Guest, true),
        (
            DEFAULT_MEMBERSHIP_GRACE_PERIOD + nanosecond,
            UserRole::Guest,
            true,
        ),
    ];
    for (age, role, stale) in cases {
        let decision = evaluate_cached_membership_permissions(
            now,
            Some(now - age),
            UserRole::Admin,
            false,
            None,
            false,
        );
        assert_eq!(decision.effective_role, Some(role), "age {age}");
        assert_eq!(decision.stale, stale, "age {age}");
        assert!(!decision.override_active);
    }
}

#[test]
fn discord_resilience_invalid_snapshot_retains_guest_access() {
    let now = now();
    for snapshot in [None, Some(now + Duration::nanoseconds(1))] {
        let decision = evaluate_cached_membership_permissions(
            now,
            snapshot,
            UserRole::Admin,
            false,
            Some(now + Duration::days(30)),
            false,
        );
        assert_eq!(decision.effective_role, Some(UserRole::Guest));
        assert!(decision.stale);
        assert!(!decision.override_active);
    }
}

#[test]
fn discord_resilience_override_expiry_is_exclusive() {
    let now = now();
    for (expires_at, active) in [
        (now - Duration::nanoseconds(1), false),
        (now, false),
        (now + Duration::nanoseconds(1), true),
    ] {
        let decision = evaluate_cached_membership_permissions(
            now,
            Some(now - DEFAULT_MEMBERSHIP_GRACE_PERIOD),
            UserRole::Admin,
            false,
            Some(expires_at),
            false,
        );
        assert_eq!(decision.override_active, active);
        assert_eq!(
            decision.effective_role,
            Some(if active {
                UserRole::Admin
            } else {
                UserRole::Guest
            })
        );
        assert!(decision.stale);
    }

    let fresh = evaluate_cached_membership_permissions(
        now,
        Some(now),
        UserRole::Admin,
        false,
        Some(now + Duration::days(1)),
        false,
    );
    assert!(
        !fresh.override_active,
        "fresh grants do not rely on an override"
    );
}

#[test]
fn discord_resilience_bans_and_departures_override_cached_grants() {
    let now = now();
    for age in [Duration::zero(), Duration::days(60)] {
        for confirmed_nonmember in [false, true] {
            let banned = evaluate_cached_membership_permissions(
                now,
                Some(now - age),
                UserRole::Admin,
                confirmed_nonmember,
                Some(now + Duration::days(1)),
                true,
            );
            assert_eq!(banned.effective_role, None);
            assert!(!banned.override_active);
        }
        let departed = evaluate_cached_membership_permissions(
            now,
            Some(now - age),
            UserRole::Admin,
            true,
            Some(now + Duration::days(1)),
            false,
        );
        assert_eq!(departed.effective_role, Some(UserRole::Guest));
        assert!(!departed.override_active);
    }
}

#[test]
fn discord_resilience_recovery_requires_verified_nonbanned_admin() {
    let now = now();
    for role in [
        UserRole::Guest,
        UserRole::Enlisted,
        UserRole::Leader,
        UserRole::MissionMaker,
        UserRole::Admin,
    ] {
        for banned in [false, true] {
            for departed in [false, true] {
                assert_eq!(
                    can_manage_sync_override(
                        now,
                        Some(now - Duration::days(365)),
                        role,
                        departed,
                        banned,
                    ),
                    role == UserRole::Admin && !departed && !banned,
                );
            }
        }
    }
    for snapshot in [None, Some(now + Duration::nanoseconds(1))] {
        assert!(!can_manage_sync_override(
            now,
            snapshot,
            UserRole::Admin,
            false,
            false,
        ));
    }
}

proptest! {
    #[test]
    fn discord_resilience_cached_privilege_lifetime_matches_age(
        age_seconds in -31_536_000_i64..31_536_000_i64,
        override_seconds in -86_400_i64..86_400_i64,
        banned in any::<bool>(),
        departed in any::<bool>(),
    ) {
        let now = now();
        let decision = evaluate_cached_membership_permissions(
            now,
            Some(now - Duration::seconds(age_seconds)),
            UserRole::Admin,
            departed,
            Some(now + Duration::seconds(override_seconds)),
            banned,
        );
        let valid_snapshot = age_seconds >= 0;
        let within_grace = age_seconds < 48 * 60 * 60;
        let overridden = valid_snapshot && !within_grace && override_seconds > 0;
        let can_retain = !departed && valid_snapshot && (within_grace || overridden);
        let expected_role = if banned {
            None
        } else if can_retain {
            Some(UserRole::Admin)
        } else {
            Some(UserRole::Guest)
        };
        prop_assert_eq!(decision.effective_role, expected_role);
        prop_assert_eq!(decision.stale, !(0..=60).contains(&age_seconds));
        prop_assert_eq!(decision.override_active, !banned && !departed && overridden);
    }
}
