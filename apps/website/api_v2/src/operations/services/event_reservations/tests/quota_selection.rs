//! Quota conservation, opening boundaries, checked arithmetic, and explicit wire limits.

use super::*;
use crate::operations::models::reservation_quota::ReservationQuotaPool;
use chrono::Duration;
use proptest::prelude::*;
use serde_json::{Value, json};

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
    crate::property_evidence::run_property(
        "generated_operation_sequences_conserve_allocations_and_capacity",
        512,
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
    );
}

#[test]
fn arbitrary_u64_usage_agrees_with_wide_integer_arithmetic() {
    crate::property_evidence::run_property(
        "arbitrary_u64_usage_agrees_with_wide_integer_arithmetic",
        512,
        &(any::<[u64; 4]>(), any::<bool>()),
        |(counts, member)| {
            let mut usage = ReservationQuotaUsage {
                member: counts[0],
                guest: counts[1],
                open: counts[2],
                legacy_unclassified: counts[3],
            };
            let before = usage;
            let total: u128 = counts.into_iter().map(u128::from).sum();
            if total <= u128::from(u64::MAX) {
                prop_assert_eq!(usage.total(), Ok(total as u64));
            } else {
                prop_assert_eq!(usage.total(), Err("quota usage overflow"));
            }
            let result = usage.reserve(&quotas([None; 3], [0; 3]), member, opening(), 0);
            if total >= u128::from(u64::MAX) {
                prop_assert_eq!(result, Err("quota usage overflow"));
                prop_assert_eq!(usage, before);
            } else {
                let preferred = if member {
                    ReservationQuotaKind::Member
                } else {
                    ReservationQuotaKind::Guest
                };
                prop_assert_eq!(result, Ok(Some(preferred)));
                prop_assert_eq!(usage.total(), Ok((total + 1) as u64));
                for kind in KINDS {
                    prop_assert_eq!(
                        usage.count(kind),
                        before.count(kind) + u64::from(kind == preferred)
                    );
                }
                prop_assert_eq!(usage.legacy_unclassified, before.legacy_unclassified);
            }
            Ok(())
        },
    );
}

#[test]
fn every_valid_u32_or_explicit_null_limit_round_trips() {
    crate::property_evidence::run_property(
        "every_valid_u32_or_explicit_null_limit_round_trips",
        512,
        &(prop::array::uniform3(prop::option::of(any::<u32>())),),
        |(limits,)| {
            let configured = quotas(limits, [0; 3]);
            let wire = serde_json::to_value(&configured).unwrap();
            let decoded: ReservationQuotas = serde_json::from_value(wire.clone()).unwrap();
            prop_assert_eq!(&decoded, &configured);
            for (index, kind) in KINDS.into_iter().enumerate() {
                prop_assert_eq!(&wire[kind.as_str()]["seats"], &json!(limits[index]));
                prop_assert_eq!(decoded.pool(kind).opens_at, opening());
            }
            Ok(())
        },
    );
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
fn preferred_pool_is_used_before_open_and_reused_after_release() {
    for member in [false, true] {
        let preferred = if member {
            ReservationQuotaKind::Member
        } else {
            ReservationQuotaKind::Guest
        };
        let configured = quotas([Some(1); 3], [0; 3]);
        let mut usage = ReservationQuotaUsage::default();
        assert_eq!(
            usage.reserve(&configured, member, opening(), 0),
            Ok(Some(preferred))
        );
        assert_eq!(
            usage.reserve(&configured, member, opening(), 0),
            Ok(Some(ReservationQuotaKind::Open))
        );
        let before = usage;
        assert_eq!(usage.reserve(&configured, member, opening(), 0), Ok(None));
        assert_eq!(usage, before);
        usage.release(preferred).unwrap();
        assert_eq!(
            usage.reserve(&configured, member, opening(), 0),
            Ok(Some(preferred))
        );
        assert_eq!(usage.total(), Ok(2));
    }
}

#[test]
fn fallback_respects_the_open_pool_opening_and_closed_pool_does_not_grant_capacity() {
    for member in [false, true] {
        let configured = quotas([Some(0), Some(0), Some(1)], [0, 0, 1]);
        let mut usage = ReservationQuotaUsage::default();
        assert_eq!(usage.reserve(&configured, member, opening(), 0), Ok(None));
        assert_eq!(usage, ReservationQuotaUsage::default());
        assert_eq!(
            usage.reserve(&configured, member, opening() + Duration::seconds(1), 0),
            Ok(Some(ReservationQuotaKind::Open))
        );
    }
}

#[test]
fn members_and_guests_never_borrow_the_other_class_even_when_it_is_uncapped() {
    for member in [false, true] {
        let limits = if member {
            [Some(0), None, Some(0)]
        } else {
            [None, Some(0), Some(0)]
        };
        let mut usage = ReservationQuotaUsage::default();
        assert_eq!(
            usage.reserve(&quotas(limits, [0; 3]), member, opening(), 0),
            Ok(None)
        );
        assert_eq!(usage, ReservationQuotaUsage::default());
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
fn explicit_uncapped_pools_still_obey_the_event_limit() {
    let configured = quotas([None; 3], [0; 3]);
    let mut usage = ReservationQuotaUsage::default();
    for member in [true, false] {
        assert!(
            usage
                .reserve(&configured, member, opening(), 2)
                .unwrap()
                .is_some()
        );
    }
    let before = usage;
    assert_eq!(usage.reserve(&configured, true, opening(), 2), Ok(None));
    assert_eq!(usage, before);
    assert_eq!(
        usage.reserve(&configured, true, opening(), 0),
        Ok(Some(ReservationQuotaKind::Member))
    );
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

#[test]
fn capacity_reductions_cannot_relabel_allocations_into_another_pool() {
    let usage = ReservationQuotaUsage {
        member: 3,
        guest: 2,
        open: 1,
        legacy_unclassified: 0,
    };
    let before = usage;
    assert_eq!(
        usage.validate_limits(&quotas([Some(3), Some(2), Some(1)], [0; 3]), 6),
        Ok(())
    );
    assert_eq!(
        usage.validate_limits(&quotas([None; 3], [0; 3]), 5),
        Err("event capacity is below current allocation")
    );
    for limits in [
        [Some(2), None, None],
        [None, Some(1), None],
        [None, None, Some(0)],
    ] {
        assert_eq!(
            usage.validate_limits(&quotas(limits, [0; 3]), 0),
            Err("quota capacity is below current allocation")
        );
    }
    assert_eq!(usage.validate_limits(&quotas([None; 3], [0; 3]), 0), Ok(()));
    assert_eq!(usage, before);
}

#[test]
fn underflow_and_both_forms_of_overflow_leave_usage_unchanged() {
    let configured = quotas([None; 3], [0; 3]);
    for kind in KINDS {
        let mut empty = ReservationQuotaUsage::default();
        assert_eq!(empty.release(kind), Err("quota usage underflow"));
        assert_eq!(empty, ReservationQuotaUsage::default());
    }
    for initial in [
        ReservationQuotaUsage {
            member: u64::MAX,
            guest: 0,
            open: 0,
            legacy_unclassified: 0,
        },
        ReservationQuotaUsage {
            member: u64::MAX - 1,
            guest: 1,
            open: 0,
            legacy_unclassified: 0,
        },
        ReservationQuotaUsage {
            member: u64::MAX,
            guest: 1,
            open: 0,
            legacy_unclassified: 0,
        },
        ReservationQuotaUsage {
            member: 0,
            guest: u64::MAX,
            open: 1,
            legacy_unclassified: 0,
        },
    ] {
        for member in [false, true] {
            let mut usage = initial;
            assert_eq!(
                usage.reserve(&configured, member, opening(), 0),
                Err("quota usage overflow")
            );
            assert_eq!(usage, initial);
        }
    }
}

fn pool_wire(seats: Value) -> Value {
    json!({"seats": seats, "opens_at": "2026-09-22T12:00:00.123456789Z"})
}

#[test]
fn wire_limits_require_presence_and_distinguish_zero_from_explicit_null() {
    assert!(
        serde_json::from_value::<ReservationQuotaPool>(json!({"opens_at": "2026-09-22T12:00:00Z"}))
            .is_err()
    );
    for limit in [Some(0), Some(1), Some(u32::MAX), None] {
        let wire = pool_wire(json!(limit));
        let parsed: ReservationQuotaPool = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.seats, limit);
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
}

#[test]
fn wire_limits_reject_negative_overflow_float_boolean_and_wrong_shapes() {
    for value in [
        json!(-1),
        json!(u64::from(u32::MAX) + 1),
        json!(u64::MAX),
        json!(1.0),
        json!(0.5),
        json!(true),
        json!(false),
        json!("1"),
        json!([]),
        json!({}),
    ] {
        assert!(
            serde_json::from_value::<ReservationQuotaPool>(pool_wire(value.clone())).is_err(),
            "accepted {value}"
        );
    }
    for nonfinite in ["NaN", "Infinity", "-Infinity"] {
        let raw = format!(r#"{{"seats":{nonfinite},"opens_at":"2026-09-22T12:00:00Z"}}"#);
        assert!(serde_json::from_str::<ReservationQuotaPool>(&raw).is_err());
    }
}

#[test]
fn all_pools_and_opening_times_are_required_and_unknown_fields_are_rejected() {
    let valid = json!({"member": pool_wire(json!(1)), "guest": pool_wire(json!(0)), "open": pool_wire(Value::Null)});
    for missing in ["member", "guest", "open"] {
        let mut wire = valid.clone();
        wire.as_object_mut().unwrap().remove(missing);
        assert!(serde_json::from_value::<ReservationQuotas>(wire).is_err());
        let mut wire = valid.clone();
        wire[missing] = Value::Null;
        assert!(serde_json::from_value::<ReservationQuotas>(wire).is_err());
    }
    let mut extra = valid.clone();
    extra["allow_overflow"] = json!(true);
    assert!(serde_json::from_value::<ReservationQuotas>(extra).is_err());
    for name in ["member", "guest", "open"] {
        let mut extra = valid.clone();
        extra[name]["capacity"] = json!(10);
        assert!(serde_json::from_value::<ReservationQuotas>(extra).is_err());
    }
    for wire in [
        Value::Null,
        json!(false),
        json!(0),
        json!([]),
        json!({"seats": 1}),
    ] {
        assert!(serde_json::from_value::<ReservationQuotaPool>(wire).is_err());
    }
    for timestamp in [
        Value::Null,
        json!(false),
        json!(0),
        json!(""),
        json!("2026-09-22"),
        json!("2026-09-22T12:00:00"),
        json!("2026-02-30T12:00:00Z"),
    ] {
        let mut wire = pool_wire(json!(1));
        wire["opens_at"] = timestamp;
        assert!(serde_json::from_value::<ReservationQuotaPool>(wire).is_err());
    }
}

#[test]
fn offset_timestamps_normalize_to_the_same_precise_utc_opening() {
    let parsed: ReservationQuotaPool = serde_json::from_value(json!({
        "seats": 1, "opens_at": "2026-09-22T14:00:00.123456789+02:00"
    }))
    .unwrap();
    assert_eq!(parsed.opens_at, opening());
    assert_eq!(serde_json::to_value(parsed).unwrap(), pool_wire(json!(1)));
}

#[test]
fn quota_kind_wire_values_are_exact_snake_case_and_reject_unknown_variants() {
    for (kind, spelling) in KINDS.into_iter().zip(["member", "guest", "open"]) {
        assert_eq!(kind.as_str(), spelling);
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(spelling));
        assert_eq!(
            serde_json::from_value::<ReservationQuotaKind>(json!(spelling)).unwrap(),
            kind
        );
    }
    for invalid in [
        Value::Null,
        json!(false),
        json!(0),
        json!("Member"),
        json!("members"),
        json!(""),
        json!({}),
    ] {
        assert!(serde_json::from_value::<ReservationQuotaKind>(invalid).is_err());
    }
}

#[test]
fn refusals_name_the_earliest_unopened_pool_with_remaining_places() {
    // Guest pool has places but opens in 60 seconds; the open pool is closed with zero places.
    let configured = quotas([None, Some(4), Some(0)], [0, 60, 0]);
    let mut usage = ReservationQuotaUsage::default();
    assert_eq!(
        usage.decide(&configured, false, opening(), 0),
        Ok(QuotaDecision::NotYetOpen {
            quota_kind: ReservationQuotaKind::Guest,
            opens_at: opening() + Duration::seconds(60),
        })
    );
    // A full member pool overflows to an open pool that opens later.
    let overflow = quotas([Some(1), Some(0), Some(3)], [0, 0, 30]);
    let mut members = ReservationQuotaUsage {
        member: 1,
        ..Default::default()
    };
    assert_eq!(
        members.decide(&overflow, true, opening(), 0),
        Ok(QuotaDecision::NotYetOpen {
            quota_kind: ReservationQuotaKind::Open,
            opens_at: opening() + Duration::seconds(30),
        })
    );
    assert_eq!(
        members.decide(&overflow, true, opening() + Duration::seconds(30), 0),
        Ok(QuotaDecision::Reserved(ReservationQuotaKind::Open))
    );
    // Closed pools and a reached event limit are exhausted, not pending.
    let closed = quotas([Some(0), Some(0), Some(0)], [0; 3]);
    assert_eq!(
        ReservationQuotaUsage::default().decide(&closed, true, opening(), 0),
        Ok(QuotaDecision::Exhausted)
    );
    let mut legacy = ReservationQuotaUsage {
        legacy_unclassified: 2,
        ..Default::default()
    };
    assert_eq!(
        legacy.decide(&quotas([None; 3], [0; 3]), true, opening(), 2),
        Ok(QuotaDecision::Exhausted)
    );
    assert_eq!(legacy.total(), Ok(2));
}
