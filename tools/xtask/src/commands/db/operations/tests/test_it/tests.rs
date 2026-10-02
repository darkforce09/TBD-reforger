use super::*;
use std::cell::Cell;
use std::collections::HashSet;

#[test]
fn cleanup_uses_an_exact_prefix_and_suffix() {
    assert_eq!(
        reap_select("rust_it"),
        "SELECT datname FROM pg_database WHERE datname = 'rust_it' OR \
         (left(datname, 8) = 'rust_it_' AND right(datname, 3) = '_it' \
         AND length(datname) > 11)"
    );
    assert!(!reap_select("tbd_gate_a_b").contains("LIKE"));
    assert!(reap_select("tbd_gate_a_b").contains("= 'tbd_gate_a_b_'"));
    assert!(reap_select("tbd_gate_'_it").contains("= 'tbd_gate_''_it'"));
}

#[test]
fn the_guard_refuses_the_live_database_and_nonidentifiers() {
    let message = validate_label("tbd_reforger").expect_err("live database must be refused");
    assert!(message.contains("scratch allow-list"));
    assert!(message.contains("tbd_reforger"));
    for label in [
        "",
        "postgres",
        "test%;drop_it",
        "tbd_gate_';DROP",
        "test\\_it",
        "tést_it",
    ] {
        assert!(validate_label(label).is_err(), "accepted {label:?}");
    }
    for label in [
        IT_BASE_DB,
        "tbd_gate_a_b",
        "TBD_long_it",
        "12_probe",
        "x_cold",
    ] {
        validate_label(label).expect("safe operator label");
    }
}

#[test]
fn random_namespace_is_bounded_and_retains_all_random_bits() {
    let long_label = format!("tbd_gate_{}", "long_label_".repeat(100));
    let first = namespace_from_random(&long_label, [0; 16]);
    let second = namespace_from_random(&long_label, [255; 16]);
    assert!(first.len() <= 42);
    assert!(is_scratch_identifier(&first));
    assert!(first.contains(&"00".repeat(16)));
    assert!(second.contains(&"ff".repeat(16)));
    assert_ne!(first, second);

    // The suite-name truncation contract preserves the complete ownership prefix.
    let full_suite = format!("{first}_{}_it", "suite".repeat(40));
    let hashed_suite = format!("{}_0123456789abcdef_it", &full_suite[..43]);
    assert_eq!(hashed_suite.len(), 63);
    assert!(owns_database(&first, &hashed_suite));
    assert!(!owns_database(&second, &hashed_suite));
}

#[test]
fn repeated_invocations_have_independent_namespaces() {
    let names: HashSet<_> = (0..128)
        .map(|_| invocation_database_name("rust_it").unwrap())
        .collect();
    assert_eq!(names.len(), 128);
    assert!(
        names
            .iter()
            .all(|name| name.len() <= 42 && is_scratch_identifier(name))
    );
}

#[test]
fn cleanup_rejects_prefix_lookalikes_and_malformed_query_rows() {
    let base = "tbd_gate_a_b";
    for owned in [base, "tbd_gate_a_b_suite_it", "tbd_gate_a_b_other_suite_it"] {
        assert!(owns_database(base, owned), "rejected {owned:?}");
    }
    for foreign in [
        "tbd_gate_aXb_suite_it",
        "tbd_gate_a_bother_suite_it",
        "tbd_gate_a_b__it",
        "tbd_gate_a_b_suite",
        "tbd_gate_a_b_suite_it;DROP",
        "tbd_reforger",
        " tbd_gate_a_b_suite_it",
        "tbd_gate_a_b_suite_it ",
    ] {
        assert!(!owns_database(base, foreign), "accepted {foreign:?}");
    }
    assert!(!owns_database(
        base,
        &format!("{base}_{}_it", "x".repeat(64))
    ));
    assert!(!owns_database("tbd_gate_';DROP", "tbd_gate_suite_it"));
}

#[test]
fn cargo_spawn_error_still_runs_cleanup() {
    let cleaned = Cell::new(false);
    let error = run_with_cleanup(
        || Err(std::io::Error::from(std::io::ErrorKind::NotFound).into()),
        || {
            cleaned.set(true);
            Ok(0)
        },
    )
    .unwrap_err();
    assert!(cleaned.get());
    assert!(error.downcast_ref::<std::io::Error>().is_some());
}

#[test]
fn cleanup_failures_never_report_success_and_preserve_both_errors() {
    assert_eq!(run_with_cleanup(|| Ok(0), || Ok(3)).unwrap(), 3);
    assert_eq!(run_with_cleanup(|| Ok(101), || Ok(3)).unwrap(), 101);
    let error = run_with_cleanup(
        || Err(anyhow::anyhow!("cargo unavailable")),
        || Err(anyhow::anyhow!("database unavailable")),
    )
    .unwrap_err();
    let details = format!("{error:#}");
    assert!(details.contains("cargo unavailable"));
    assert!(details.contains("database unavailable"));
    assert!(run_with_cleanup(|| Ok(0), || Err(anyhow::anyhow!("cleanup failed"))).is_err());
}

#[test]
fn a_failing_suite_still_reports_its_own_rc() {
    assert_eq!(join_rc(101, 0), 101);
    assert_eq!(join_rc(0, 1), 1);
    assert_eq!(join_rc(0, 0), 0);
}

#[test]
fn the_complete_suite_keeps_the_canonical_cargo_arguments() {
    let arguments = cargo_test_arguments(&TestSelection::default()).unwrap();
    assert_eq!(
        arguments,
        ["test", "--locked", "--no-fail-fast", "--", "--show-output"]
    );
    assert!(TestSelection::default().is_complete_suite());
}

#[test]
fn a_narrowed_selection_forwards_binaries_library_and_filter() {
    let selection = TestSelection {
        binaries: vec!["waitlist_promotion_transactions".into(), "seatless".into()],
        library: true,
        name_filter: Some("operations::services::event_reservations".into()),
    };
    assert!(!selection.is_complete_suite());
    assert_eq!(
        cargo_test_arguments(&selection).unwrap(),
        [
            "test",
            "--locked",
            "--no-fail-fast",
            "--lib",
            "--test",
            "waitlist_promotion_transactions",
            "--test",
            "seatless",
            "--",
            "--show-output",
            "operations::services::event_reservations",
        ]
    );
}

#[test]
fn selectors_reject_option_injection_and_malformed_text() {
    for binary in ["", "--release", "a b", "a;b", "a:b", "é"] {
        let selection = TestSelection {
            binaries: vec![binary.into()],
            ..TestSelection::default()
        };
        assert!(
            cargo_test_arguments(&selection).is_err(),
            "accepted {binary:?}"
        );
    }
    for filter in ["", "--ignored", "a b", "x".repeat(129).as_str()] {
        let selection = TestSelection {
            name_filter: Some(filter.into()),
            ..TestSelection::default()
        };
        assert!(
            cargo_test_arguments(&selection).is_err(),
            "accepted {filter:?}"
        );
    }
}
