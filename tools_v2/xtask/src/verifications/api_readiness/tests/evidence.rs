use super::*;

fn fixture() -> (Check, Receipt, String) {
    let output =
        "test guest_sign_in ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored".to_owned();
    let check = Check {
        id: "guests".into(),
        class: EvidenceClass::Implementation,
        command: Some(vec!["cargo".into(), "test".into()]),
        timeout_seconds: 60,
        minimum_cases: 1,
        success_marker: "test result: ok.".into(),
        properties: Vec::new(),
        case_pattern: r"(?m)^test guest_sign_in \.\.\. ok$".into(),
    };
    let receipt = Receipt {
        version: 1,
        check_id: "guests".into(),
        class: EvidenceClass::Implementation,
        source_sha256: "source".into(),
        configuration_sha256: "configuration".into(),
        command: vec!["cargo".into(), "test".into()],
        tool_versions: vec!["rustc test".into()],
        started_unix_seconds: 100,
        duration_milliseconds: 10,
        exit_code: 0,
        output_file: "guests.log".into(),
        output_sha256: fingerprint::digest(output.as_bytes()),
        environment: Vec::new(),
        observations: None,
        property_runs: Vec::new(),
    };
    (check, receipt, output)
}

#[test]
fn current_successful_cases_are_accepted() {
    let (check, receipt, output) = fixture();
    assert_eq!(
        evidence::validate(&check, &receipt, &output, "source", "configuration", 101).unwrap(),
        1
    );
}

#[test]
fn actual_register_patterns_recognize_cargo_success_cases_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let acceptance = register::read(&root).expect("read and validate actual acceptance register");

    for (check_id, names) in [
        (
            "backend_regression",
            [
                "identity_linking_confirm",
                "identity_and_access::tests::session_rotation_2",
            ],
        ),
        (
            "readiness_self_tests",
            [
                "verifications::api_readiness::tests::current_successful_cases_are_accepted",
                "verifications::api_readiness::tests::nested::recognizes_case_2",
            ],
        ),
    ] {
        let check = acceptance
            .checks
            .iter()
            .find(|check| check.id == check_id)
            .expect("acceptance register contains required aggregate check");
        let pattern = regex::Regex::new(&check.case_pattern).expect("compile registered pattern");
        let output = format!(
            "running 2 tests\ntest {} ... ok\ntest {} ... ok\n\
             test result: ok. 2 passed; 0 failed; 0 ignored\n",
            names[0], names[1]
        );
        assert_eq!(
            pattern.find_iter(&output).count(),
            2,
            "{check_id} must recognize actual cargo success lines"
        );

        for unrelated in [
            "test result: ok. 2 passed; 0 failed; 0 ignored".to_owned(),
            format!("case {} ... ok", names[0]),
            format!("log: test {} ... ok", names[0]),
            format!("test {} xxx ok", names[0]),
            format!("test {} ... FAILED", names[0]),
            format!("test {} ... ignored", names[0]),
            format!("test {} ... okay", names[0]),
            format!("test {} ... ok trailing text", names[0]),
        ] {
            assert!(
                !pattern.is_match(&unrelated),
                "{check_id} must reject pseudo-success output: {unrelated:?}"
            );
        }

        if check_id == "readiness_self_tests" {
            assert!(
                !pattern.is_match("test unrelated::tests::current_successful_cases ... ok"),
                "readiness evidence must come from the readiness test namespace"
            );
        } else {
            assert!(
                !pattern.is_match("test identity_linking_confirm\ncontinuation ... ok"),
                "a cargo case cannot extend across a newline"
            );
        }
    }
}

#[test]
fn empty_or_unrelated_success_does_not_satisfy_a_requirement() {
    let (check, mut receipt, _) = fixture();
    for output in [
        "test result: ok. 0 passed; 0 failed",
        "test unrelated ... ok\ntest result: ok.",
    ] {
        receipt.output_sha256 = fingerprint::digest(output.as_bytes());
        assert!(
            evidence::validate(&check, &receipt, output, "source", "configuration", 101).is_err()
        );
    }
}

#[test]
fn stale_changed_failed_and_omitted_evidence_is_rejected() {
    let (check, receipt, output) = fixture();
    assert!(
        evidence::validate(&check, &receipt, &output, "changed", "configuration", 101).is_err()
    );
    assert!(
        evidence::validate(&check, &receipt, &output, "source", "configuration", 90000).is_err()
    );
    assert!(evidence::validate(&check, &receipt, &output, "source", "configuration", 99).is_err());
    let mut failed = receipt;
    failed.exit_code = 1;
    assert!(evidence::validate(&check, &failed, &output, "source", "configuration", 101).is_err());
    failed.exit_code = 0;
    let omitted = format!("{output}\nskip: TEST_DATABASE_URL unset");
    failed.output_sha256 = fingerprint::digest(omitted.as_bytes());
    assert!(evidence::validate(&check, &failed, &omitted, "source", "configuration", 101).is_err());
}

#[test]
fn evidence_cannot_claim_a_different_command_or_output() {
    let (check, mut receipt, output) = fixture();
    assert!(
        evidence::validate(&check, &receipt, "altered", "source", "configuration", 101).is_err()
    );
    receipt.command = vec!["true".into()];
    assert!(evidence::validate(&check, &receipt, &output, "source", "configuration", 101).is_err());
}

#[test]
fn external_operational_evidence_requires_environment_identity() {
    let (mut check, mut receipt, output) = fixture();
    check.class = EvidenceClass::Operational;
    receipt.class = EvidenceClass::Operational;
    assert!(evidence::validate(&check, &receipt, &output, "source", "configuration", 101).is_err());
    receipt.environment = vec!["staging: fixture-sha256".into()];
    assert!(
        evidence::validate(&check, &receipt, &output, "source", "configuration", 101).is_err(),
        "environment name alone cannot satisfy operational acceptance"
    );
}

#[test]
fn register_paths_cannot_escape_the_repository() {
    for path in ["", "../outside", "/absolute", "valid/../../outside"] {
        assert!(!register::relative_path(path));
    }
    assert!(register::relative_path("apps/website/api_v2"));
}

fn scratch() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "api-readiness-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    path
}

#[test]
fn evidence_writes_do_not_follow_symlinks_or_modify_hardlinked_targets() {
    use std::os::unix::fs::symlink;
    let directory = scratch();
    let outside = directory.join("outside");
    std::fs::write(&outside, "preserve").unwrap();
    symlink(&outside, directory.join("receipt.json")).unwrap();
    assert!(evidence_storage::write(&directory, "receipt.json", b"replace").is_err());
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "preserve");
    std::fs::remove_file(directory.join("receipt.json")).unwrap();
    std::fs::hard_link(&outside, directory.join("receipt.json")).unwrap();
    evidence_storage::write(&directory, "receipt.json", b"replace").unwrap();
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "preserve");
    assert_eq!(
        std::fs::read_to_string(directory.join("receipt.json")).unwrap(),
        "replace"
    );
    symlink(&directory, directory.join("alias")).unwrap();
    assert!(evidence_storage::write(&directory.join("alias"), "other.json", b"bad").is_err());
    assert!(evidence_storage::write(&directory, "../outside.json", b"bad").is_err());
    assert!(std::fs::read_dir(&directory).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn configuration_fingerprint_distinguishes_missing_empty_and_changed_inputs() {
    let root = scratch();
    let missing = fingerprint::configuration(&root).unwrap();
    std::fs::write(root.join(".env"), "").unwrap();
    let empty = fingerprint::configuration(&root).unwrap();
    std::fs::write(root.join(".env"), "APP_ENV=development\n").unwrap();
    let configured = fingerprint::configuration(&root).unwrap();
    assert_ne!(missing, empty);
    assert_ne!(empty, configured);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ignored_tests_with_reasons_and_unreported_ignored_summaries_fail() {
    let (check, mut receipt, output) = fixture();
    for suffix in [
        "\ntest acceptance ... ignored, database unavailable",
        "\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured",
    ] {
        let altered = format!("{output}{suffix}");
        receipt.output_sha256 = fingerprint::digest(altered.as_bytes());
        assert!(
            evidence::validate(&check, &receipt, &altered, "source", "configuration", 101).is_err()
        );
    }
}

#[test]
fn operational_load_requires_all_recorded_acceptance_conditions() {
    let observations = operational::Observations::Load {
        duration_seconds: 1800,
        member_accounts: 1000,
        minimum_concurrent_clients: 100,
        completed_requests: 36000,
        unexpected_errors: 0,
        json_read_count: 25000,
        json_write_count: 11000,
        p95_json_read_ms: 500.0,
        p95_json_write_ms: 1000.0,
        workload_sha256: "a".repeat(64),
        hardware: "recorded host".into(),
        network: "recorded network".into(),
    };
    assert!(operational::validate("staging_load", &observations).is_ok());
    let value = serde_json::to_value(&observations).unwrap();
    for (field, replacement) in [
        ("duration_seconds", serde_json::json!(1)),
        ("member_accounts", serde_json::json!(1)),
        ("minimum_concurrent_clients", serde_json::json!(1)),
        ("completed_requests", serde_json::json!(1)),
        ("unexpected_errors", serde_json::json!(1)),
        ("p95_json_read_ms", serde_json::json!(501)),
        ("p95_json_write_ms", serde_json::json!(1001)),
        ("hardware", serde_json::json!("")),
    ] {
        let mut invalid = value.clone();
        invalid[field] = replacement;
        let observation = serde_json::from_value(invalid).unwrap();
        assert!(
            operational::validate("staging_load", &observation).is_err(),
            "accepted deficient {field}"
        );
    }
}

#[test]
fn duplicate_output_cannot_substitute_for_distinct_acceptance_cases() {
    let (mut check, mut receipt, _) = fixture();
    check.minimum_cases = 2;
    let output = "test guest_sign_in ... ok\ntest guest_sign_in ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored;";
    receipt.output_sha256 = fingerprint::digest(output.as_bytes());
    assert!(evidence::validate(&check, &receipt, output, "source", "configuration", 101).is_err());
}
