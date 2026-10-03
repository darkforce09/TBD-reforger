//! Unit tests for the bucket spend's schedule: when it reads again, when it stops, and which starts
//! it refuses.

use super::{
    LATE_START_TOLERANCE_MS, MAXIMUM_START_DELAY_MS, NextStep, SpendStop, check_start, next_step,
    saw_bucket_spent,
};
use crate::discord_member_probe::member_read::{MemberOutcome, RateLimitHeaders};

const NOW: u64 = 1_900_000_000_000;
const HOLD_UNTIL: u64 = NOW + 3000;

fn member() -> MemberOutcome {
    MemberOutcome::Member {
        roles: vec!["1".to_owned()],
    }
}

fn headers(remaining: Option<u32>, reset_after_ms: Option<u64>) -> RateLimitHeaders {
    RateLimitHeaders {
        remaining,
        reset_after_ms,
        ..RateLimitHeaders::default()
    }
}

#[test]
fn staging_fixtures_bucket_spend_drains_while_the_bucket_has_room() {
    let step = next_step(
        &member(),
        &headers(Some(3), Some(900)),
        NOW,
        HOLD_UNTIL,
        2,
        50,
    );
    assert_eq!(step, NextStep::SendNow);
    let unknown = next_step(
        &MemberOutcome::Nonmember,
        &headers(None, None),
        NOW,
        HOLD_UNTIL,
        2,
        50,
    );
    assert_eq!(unknown, NextStep::SendNow);
}

#[test]
fn staging_fixtures_bucket_spend_waits_for_the_reset_once_spent() {
    let spent = next_step(
        &member(),
        &headers(Some(0), Some(900)),
        NOW,
        HOLD_UNTIL,
        5,
        50,
    );
    assert_eq!(spent, NextStep::SendAt(NOW + 900 + 25));
    let limited = MemberOutcome::RateLimited {
        retry_after_ms: Some(400),
        global: false,
    };
    let step = next_step(&limited, &headers(Some(0), None), NOW, HOLD_UNTIL, 6, 50);
    assert_eq!(step, NextStep::SendAt(NOW + 400 + 25));
}

#[test]
fn staging_fixtures_bucket_spend_stops_at_the_hold_the_cap_or_an_unavailable_discord() {
    let late_reset = next_step(
        &member(),
        &headers(Some(0), Some(5000)),
        NOW,
        HOLD_UNTIL,
        5,
        50,
    );
    assert_eq!(late_reset, NextStep::Stop(SpendStop::HoldEnded));
    let unknown_delay = MemberOutcome::RateLimited {
        retry_after_ms: None,
        global: false,
    };
    let step = next_step(&unknown_delay, &headers(None, None), NOW, HOLD_UNTIL, 5, 50);
    assert_eq!(step, NextStep::Stop(SpendStop::HoldEnded));
    let past_hold = next_step(
        &member(),
        &headers(Some(3), None),
        HOLD_UNTIL,
        HOLD_UNTIL,
        5,
        50,
    );
    assert_eq!(past_hold, NextStep::Stop(SpendStop::HoldEnded));
    let capped = next_step(&member(), &headers(Some(3), None), NOW, HOLD_UNTIL, 50, 50);
    assert_eq!(capped, NextStep::Stop(SpendStop::RequestCap));
    let refused = MemberOutcome::Unavailable {
        reason: "Discord refused the bot token",
    };
    let step = next_step(&refused, &headers(Some(3), None), NOW, HOLD_UNTIL, 1, 50);
    assert_eq!(step, NextStep::Stop(SpendStop::DiscordUnavailable));
}

#[test]
fn staging_fixtures_bucket_spend_sees_the_bucket_spent_by_remaining_zero_or_a_429() {
    assert!(saw_bucket_spent(&member(), &headers(Some(0), None)));
    let limited = MemberOutcome::RateLimited {
        retry_after_ms: None,
        global: false,
    };
    assert!(saw_bucket_spent(&limited, &headers(None, None)));
    assert!(!saw_bucket_spent(&member(), &headers(Some(1), None)));
    assert!(!saw_bucket_spent(&member(), &headers(None, None)));
}

#[test]
fn staging_fixtures_bucket_spend_refuses_a_start_long_past_or_far_ahead() {
    assert!(check_start(NOW, NOW).is_ok());
    assert!(check_start(NOW - LATE_START_TOLERANCE_MS, NOW).is_ok());
    assert!(check_start(NOW - LATE_START_TOLERANCE_MS - 1, NOW).is_err());
    assert!(check_start(NOW + MAXIMUM_START_DELAY_MS, NOW).is_ok());
    assert!(check_start(NOW + MAXIMUM_START_DELAY_MS + 1, NOW).is_err());
}
