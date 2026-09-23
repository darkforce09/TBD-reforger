//! Property-test configuration parsing never mutates the process environment.

use super::PropertyTestConfiguration;
use std::ffi::OsStr;

fn parse(cases: Option<&str>, seed: Option<&str>) -> anyhow::Result<PropertyTestConfiguration> {
    PropertyTestConfiguration::from_values(cases.map(OsStr::new), seed.map(OsStr::new))
}

#[test]
fn absent_values_select_the_versioned_reproducible_seed() {
    assert_eq!(parse(None, None).unwrap().rng_seed, 2_026_092_201);
    assert_eq!(parse(None, None).unwrap(), parse(None, None).unwrap());
}

#[test]
fn every_present_case_override_is_rejected() {
    for cases in [
        "",
        "0",
        "1",
        "256",
        "512",
        "suite-defined",
        "-1",
        " ",
        "invalid",
    ] {
        for seed in [None, Some("42")] {
            let error = parse(Some(cases), seed).unwrap_err();
            assert!(error.to_string().contains("PROPTEST_CASES"));
        }
    }
}

#[test]
fn explicit_decimal_seeds_accept_the_entire_u64_range() {
    for (raw, expected) in [
        ("0", 0),
        ("1", 1),
        ("2026092201", 2_026_092_201),
        ("00042", 42),
        ("18446744073709551615", u64::MAX),
    ] {
        assert_eq!(parse(None, Some(raw)).unwrap().rng_seed, expected);
    }
}

#[test]
fn malformed_empty_and_overflowing_seeds_fail_instead_of_using_defaults() {
    for seed in [
        "",
        " ",
        " 42",
        "42 ",
        "42\n",
        "+1",
        "-1",
        "1.0",
        "1e3",
        "0x10",
        "1_000",
        "random",
        "NaN",
        "１２",
        "18446744073709551616",
        "9999999999999999999999999999999999999999",
    ] {
        let error = parse(None, Some(seed)).unwrap_err();
        assert!(error.to_string().contains("PROPTEST_RNG_SEED"));
    }
}

#[test]
fn markers_and_receipt_environment_have_exact_stable_spellings() {
    for seed in [0, 2_026_092_201, u64::MAX] {
        let configuration = PropertyTestConfiguration { rng_seed: seed };
        assert_eq!(
            configuration.marker(),
            format!("property-test-configuration: rng_seed={seed}; cases=suite-defined")
        );
        assert_eq!(
            configuration.receipt_environment(),
            vec![
                format!("PROPTEST_RNG_SEED={seed}"),
                "PROPTEST_CASES=suite-defined".to_owned(),
            ]
        );
        assert!(!configuration.marker().contains('\n'));
    }
}

#[test]
fn noncanonical_decimal_input_emits_the_effective_seed() {
    let configuration = parse(None, Some("00042")).unwrap();
    assert_eq!(
        configuration.marker(),
        "property-test-configuration: rng_seed=42; cases=suite-defined"
    );
    assert_eq!(
        configuration.receipt_environment()[0],
        "PROPTEST_RNG_SEED=42"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_seed_is_rejected_without_fallback() {
    use std::os::unix::ffi::OsStrExt;
    let error = PropertyTestConfiguration::from_values(None, Some(OsStr::from_bytes(b"\xff42")))
        .unwrap_err();
    assert!(error.to_string().contains("PROPTEST_RNG_SEED"));
}

#[cfg(unix)]
#[test]
fn non_utf8_case_override_is_still_a_present_override() {
    use std::os::unix::ffi::OsStrExt;
    let error =
        PropertyTestConfiguration::from_values(Some(OsStr::from_bytes(b"\xff")), None).unwrap_err();
    assert!(error.to_string().contains("PROPTEST_CASES"));
}

#[cfg(windows)]
#[test]
fn invalid_unicode_environment_values_are_rejected() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let invalid = OsString::from_wide(&[0xd800]);
    let seed = PropertyTestConfiguration::from_values(None, Some(invalid.as_os_str()));
    assert!(seed.unwrap_err().to_string().contains("PROPTEST_RNG_SEED"));
    let cases = PropertyTestConfiguration::from_values(Some(invalid.as_os_str()), None);
    assert!(cases.unwrap_err().to_string().contains("PROPTEST_CASES"));
}
