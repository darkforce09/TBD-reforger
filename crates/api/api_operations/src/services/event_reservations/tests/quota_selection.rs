//! Quota conservation and the opening and capacity boundaries.

use super::*;
use crate::models::reservation_quota::ReservationQuotaPool;
use chrono::Duration;
use proptest::prelude::*;

const KINDS: [ReservationQuotaKind; 3] = [
    ReservationQuotaKind::Member,
    ReservationQuotaKind::Guest,
    ReservationQuotaKind::Open,
];

fn opening() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-22T12:00:00.123456789Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn quotas(limits: [Option<u32>; 3], offsets: [i64; 3]) -> ReservationQuotas {
    let pools: Vec<_> = limits
        .into_iter()
        .zip(offsets)
        .map(|(seats, offset)| ReservationQuotaPool {
            seats,
            opens_at: opening() + Duration::seconds(offset),
        })
        .collect();
    ReservationQuotas {
        member: pools[0].clone(),
        guest: pools[1].clone(),
        open: pools[2].clone(),
    }
}

fn ledger_count(ledger: &[ReservationQuotaKind], kind: ReservationQuotaKind) -> u64 {
    ledger
        .iter()
        .filter(|allocation| **allocation == kind)
        .count() as u64
}

#[derive(Debug, Clone)]
enum Operation {
    Reserve {
        member: bool,
        at: i64,
    },
    Release(ReservationQuotaKind),
    ReleaseLegacy,
    Resize {
        limits: [Option<u32>; 3],
        event: u32,
    },
}

fn limits_strategy() -> impl Strategy<Value = [Option<u32>; 3]> {
    prop::array::uniform3(prop::option::of(0u32..12))
}

fn operation_strategy() -> impl Strategy<Value = Operation> {
    prop_oneof![
        5 => (any::<bool>(), -4i64..8).prop_map(|(member, at)| Operation::Reserve { member, at }),
        2 => prop::sample::select(KINDS.to_vec()).prop_map(Operation::Release),
        1 => Just(Operation::ReleaseLegacy),
        2 => (limits_strategy(), 0u32..30).prop_map(|(limits, event)| Operation::Resize { limits, event }),
    ]
}

#[test]
fn generated_operation_sequences_conserve_allocations_and_capacity() {
    proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 512,
        failure_persistence: None,
        ..Default::default()
    })
    .run(
        &(
            limits_strategy(),
            prop::array::uniform3(-3i64..4),
            0u32..30,
            0u64..6,
            prop::collection::vec(operation_strategy(), 1..180),
        ),
        |(initial_limits, openings, initial_event_limit, initial_legacy, operations)| {
            let mut configured = quotas(initial_limits, openings);
            // Historical allocations may already exceed a limit configured later.
            let mut event_limit = if initial_event_limit == 0 {
                0
            } else {
                initial_event_limit.max(initial_legacy as u32)
            };
            let mut usage = ReservationQuotaUsage {
                legacy_unclassified: initial_legacy,
                ..Default::default()
            };
            let mut ledger = Vec::<ReservationQuotaKind>::new();
            let mut legacy = initial_legacy;

            for operation in operations {
                let before = usage;
                match operation {
                    Operation::Reserve { member, at } => {
                        let now = opening() + Duration::seconds(at);
                        let preferred = if member {
                            ReservationQuotaKind::Member
                        } else {
                            ReservationQuotaKind::Guest
                        };
                        let available = |kind| {
                            let pool = configured.pool(kind);
                            now >= pool.opens_at
                                && pool
                                    .seats
                                    .is_none_or(|n| ledger_count(&ledger, kind) < u64::from(n))
                        };
                        let occupied = ledger.len() as u64 + legacy;
                        let expected = if event_limit != 0 && occupied >= u64::from(event_limit) {
                            None
                        } else if available(preferred) {
                            Some(preferred)
                        } else if available(ReservationQuotaKind::Open) {
                            Some(ReservationQuotaKind::Open)
                        } else {
                            None
                        };
                        let mut explained = usage;
                        let decision = explained.decide(&configured, member, now, event_limit);
                        let result = usage.reserve(&configured, member, now, event_limit);
                        prop_assert_eq!(result, Ok(expected));
                        prop_assert_eq!(explained, usage);
                        match decision {
                            Ok(QuotaDecision::Reserved(kind)) => {
                                prop_assert_eq!(Some(kind), expected)
                            }
                            Ok(QuotaDecision::NotYetOpen {
                                quota_kind,
                                opens_at,
                            }) => {
                                prop_assert_eq!(expected, None);
                                prop_assert!(now < opens_at);
                                prop_assert!(
                                    quota_kind == preferred
                                        || quota_kind == ReservationQuotaKind::Open
                                );
                                prop_assert_eq!(configured.pool(quota_kind).opens_at, opens_at);
                            }
                            Ok(QuotaDecision::Exhausted) => prop_assert_eq!(expected, None),
                            Err(error) => prop_assert!(false, "unexpected {error}"),
                        }
                        if let Some(kind) = expected {
                            prop_assert!(kind == preferred || kind == ReservationQuotaKind::Open);
                            ledger.push(kind);
                        } else {
                            prop_assert_eq!(usage, before);
                        }
                    }
                    Operation::Release(kind) => {
                        let existing = ledger.iter().position(|allocation| *allocation == kind);
                        let result = usage.release(kind);
                        if let Some(index) = existing {
                            prop_assert_eq!(result, Ok(()));
                            ledger.remove(index);
                        } else {
                            prop_assert_eq!(result, Err("quota usage underflow"));
                            prop_assert_eq!(usage, before);
                        }
                    }
                    Operation::ReleaseLegacy => {
                        let result = usage.release_legacy_unclassified();
                        if legacy > 0 {
                            prop_assert_eq!(result, Ok(()));
                            legacy -= 1;
                        } else {
                            prop_assert_eq!(result, Err("quota usage underflow"));
                            prop_assert_eq!(usage, before);
                        }
                    }
                    Operation::Resize { limits, event } => {
                        let candidate = quotas(limits, openings);
                        let occupied = ledger.len() as u64 + legacy;
                        let expected = (event == 0 || occupied <= u64::from(event))
                            && KINDS.into_iter().all(|kind| {
                                candidate
                                    .pool(kind)
                                    .seats
                                    .is_none_or(|n| ledger_count(&ledger, kind) <= u64::from(n))
                            });
                        let result = usage.validate_limits(&candidate, event);
                        prop_assert_eq!(result.is_ok(), expected);
                        prop_assert_eq!(usage, before);
                        if expected {
                            configured = candidate;
                            event_limit = event;
                        }
                    }
                }
                prop_assert_eq!(usage.total(), Ok(ledger.len() as u64 + legacy));
                prop_assert_eq!(usage.legacy_unclassified, legacy);
                for kind in KINDS {
                    prop_assert_eq!(usage.count(kind), ledger_count(&ledger, kind));
                    if let Some(limit) = configured.pool(kind).seats {
                        prop_assert!(usage.count(kind) <= u64::from(limit));
                    }
                }
                prop_assert!(
                    event_limit == 0 || ledger.len() as u64 + legacy <= u64::from(event_limit)
                );
                prop_assert_eq!(usage.validate_limits(&configured, event_limit), Ok(()));
            }
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn every_pool_opens_at_the_exact_inclusive_utc_instant() {
    for (index, kind) in KINDS.into_iter().enumerate() {
        let mut limits = [Some(0); 3];
        limits[index] = Some(1);
        let configured = quotas(limits, [0; 3]);
        for member in [false, true] {
            if (kind == ReservationQuotaKind::Member && !member)
                || (kind == ReservationQuotaKind::Guest && member)
            {
                continue;
            }
            for (delta, expected) in [(-1, None), (0, Some(kind)), (1, Some(kind))] {
                let mut usage = ReservationQuotaUsage::default();
                assert_eq!(
                    usage.reserve(
                        &configured,
                        member,
                        opening() + Duration::nanoseconds(delta),
                        0
                    ),
                    Ok(expected),
                    "{kind:?}, member={member}, nanoseconds from opening={delta}"
                );
                assert_eq!(usage.total(), Ok(u64::from(expected.is_some())));
            }
        }
    }
}

#[test]
fn zero_event_limit_is_uncapped_but_zero_pool_limits_are_closed() {
    let mut usage = ReservationQuotaUsage::default();
    let closed = quotas([Some(0); 3], [0; 3]);
    for member in [false, true] {
        assert_eq!(usage.reserve(&closed, member, opening(), 0), Ok(None));
    }
    let finite = quotas([Some(1); 3], [0; 3]);
    assert_eq!(
        usage.reserve(&finite, true, opening(), 0),
        Ok(Some(ReservationQuotaKind::Member))
    );
    assert_eq!(
        usage.reserve(&finite, false, opening(), 0),
        Ok(Some(ReservationQuotaKind::Guest))
    );
    assert_eq!(
        usage.reserve(&finite, true, opening(), 0),
        Ok(Some(ReservationQuotaKind::Open))
    );
    assert_eq!(usage.reserve(&finite, false, opening(), 0), Ok(None));
    assert_eq!(usage.total(), Ok(3));
}

#[test]
fn maximum_finite_u32_capacity_accepts_exactly_its_last_available_place() {
    let mut usage = ReservationQuotaUsage {
        member: u64::from(u32::MAX) - 1,
        ..Default::default()
    };
    let configured = quotas([Some(u32::MAX), Some(0), Some(0)], [0; 3]);
    assert_eq!(
        usage.reserve(&configured, true, opening(), u32::MAX),
        Ok(Some(ReservationQuotaKind::Member))
    );
    let full = usage;
    assert_eq!(
        usage.reserve(&configured, true, opening(), u32::MAX),
        Ok(None)
    );
    assert_eq!(usage.reserve(&configured, true, opening(), 0), Ok(None));
    assert_eq!(usage, full);
}
