//! Negative controls require the generated-case recorder to observe successful checks.
use crate::property_evidence::{collect_property, run_property};
use proptest::{prelude::*, test_runner::TestCaseError};
use std::cell::Cell;

#[test]
fn recorded_case_count_matches_observed_checks_and_seed_reproduces_inputs() {
    let checks = Cell::new(0);
    let first = collect_property("recorder_control", 64, 42, &any::<u64>(), |_| {
        checks.set(checks.get() + 1);
        Ok(())
    });
    assert_eq!(checks.get(), 64);
    let second = collect_property("recorder_control", 64, 42, &any::<u64>(), |_| Ok(()));
    assert_eq!(first, second);
    let changed = collect_property("recorder_control", 64, 43, &any::<u64>(), |_| Ok(()));
    assert_ne!(first.input_sha256, changed.input_sha256);
}

#[test]
fn zero_cases_and_failed_invariants_cannot_produce_success_evidence() {
    assert!(
        std::panic::catch_unwind(|| collect_property("zero_control", 0, 1, &Just(1), |_| Ok(())))
            .is_err()
    );
    assert!(
        std::panic::catch_unwind(
            || collect_property("broken_control", 16, 1, &Just(1), |_| Err(
                TestCaseError::fail("deliberately broken invariant")
            ))
        )
        .is_err()
    );
}

#[test]
fn rejected_cases_are_excluded_from_completed_count() {
    let rejected = Cell::new(false);
    let calls = Cell::new(0);
    let record = collect_property("rejection_control", 16, 1, &any::<u64>(), |_| {
        calls.set(calls.get() + 1);
        if !rejected.replace(true) {
            return Err(TestCaseError::reject("exercise rejection accounting"));
        }
        Ok(())
    });
    assert_eq!(calls.get(), 17);
    assert_eq!(record.executed_cases, 16);
}

#[test]
fn recorder_checked_arithmetic_obeys_wide_integer_oracle() {
    run_property(
        "recorder_checked_arithmetic_obeys_wide_integer_oracle",
        512,
        &(any::<u64>(), any::<u64>()),
        |(left, right)| {
            let mathematical_sum = u128::from(left) + u128::from(right);
            let expected = u64::try_from(mathematical_sum).ok();
            prop_assert_eq!(left.checked_add(right), expected);
            Ok(())
        },
    );
}
