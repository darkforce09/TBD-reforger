//! Property-run settings bind evidence through ordered, unambiguous raw-byte frames.

use super::*;

fn entries(values: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    values
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect()
}

fn fingerprint(environment: impl IntoIterator<Item = (OsString, OsString)>) -> String {
    let mut hash = Sha256::new();
    hash_property_environment(&mut hash, environment);
    format!("{:x}", hash.finalize())
}

#[test]
fn absent_empty_and_valued_property_settings_have_distinct_fingerprints() {
    let fingerprints = [
        fingerprint(entries(&[])),
        fingerprint(entries(&[("PROPTEST_CASES", "")])),
        fingerprint(entries(&[("PROPTEST_CASES", "0")])),
        fingerprint(entries(&[("PROPTEST_CASES", "256")])),
    ];
    assert_eq!(fingerprints.iter().collect::<BTreeSet<_>>().len(), 4);
}

#[test]
fn seed_count_algorithm_and_unknown_property_options_each_bind_the_digest() {
    let settings = entries(&[
        ("PROPTEST_RNG_SEED", "123"),
        ("PROPTEST_CASES", "512"),
        ("PROPTEST_RNG_ALGORITHM", "cc"),
        ("PROPTEST_FUTURE_OPTION", "baseline"),
    ]);
    let baseline = fingerprint(settings.clone());
    for index in 0..settings.len() {
        let mut changed = settings.clone();
        changed[index].1.push("-changed");
        assert_ne!(fingerprint(changed), baseline, "changed option {index}");

        let mut removed = settings.clone();
        removed.remove(index);
        assert_ne!(fingerprint(removed), baseline, "removed option {index}");
    }
    let mut renamed = settings.clone();
    renamed[3].0 = "PROPTEST_ANOTHER_FUTURE_OPTION".into();
    assert_ne!(fingerprint(renamed), baseline);
}

#[test]
fn enumeration_order_does_not_change_the_fingerprint() {
    let settings = entries(&[
        ("PROPTEST_RNG_SEED", "123"),
        ("PROPTEST_CASES", "512"),
        ("PROPTEST_RNG_ALGORITHM", "cc"),
    ]);
    let expected = fingerprint(settings.clone());
    // Rotations and reversed rotations enumerate all six orders of these three unique keys.
    for rotation in 0..settings.len() {
        let mut reordered = settings.clone();
        reordered.rotate_left(rotation);
        assert_eq!(fingerprint(reordered.clone()), expected);
        reordered.reverse();
        assert_eq!(fingerprint(reordered), expected);
    }
}

#[test]
fn only_the_exact_property_environment_prefix_is_selected() {
    let selected = entries(&[("PROPTEST_UNKNOWN", "included")]);
    let expected = fingerprint(selected.clone());
    let mut mixed = selected;
    mixed.extend(entries(&[
        ("PROPTEST", "missing underscore"),
        ("proptest_CASES", "wrong case"),
        ("OTHER_PROPTEST_CASES", "embedded prefix"),
        ("PATH", "unrelated"),
    ]));
    assert_eq!(fingerprint(mixed), expected);
    assert_ne!(
        fingerprint(entries(&[("PROPTEST_", "")])),
        fingerprint(entries(&[]))
    );
}

#[test]
fn length_framing_distinguishes_separator_and_newline_boundaries() {
    for (left, right) in [
        (
            entries(&[("PROPTEST_A", "x=y")]),
            entries(&[("PROPTEST_A=x", "y")]),
        ),
        (
            entries(&[("PROPTEST_A", "x|y")]),
            entries(&[("PROPTEST_A|x", "y")]),
        ),
        (
            entries(&[("PROPTEST_A", "x\ny")]),
            entries(&[("PROPTEST_A\nx", "y")]),
        ),
        (
            entries(&[("PROPTEST_A", "x\nPROPTEST_B=y")]),
            entries(&[("PROPTEST_A", "x"), ("PROPTEST_B", "y")]),
        ),
        (
            entries(&[("PROPTEST_A", "bc")]),
            entries(&[("PROPTEST_Ab", "c")]),
        ),
    ] {
        assert_ne!(fingerprint(left), fingerprint(right));
    }
}

#[test]
fn framing_matches_the_existing_length_prefixed_configuration_encoding() {
    let actual = fingerprint(entries(&[
        ("PROPTEST_RNG_SEED", "99"),
        ("PATH", "ignored"),
        ("PROPTEST_CASES", "512"),
    ]));
    // The expected bytes use a little-endian u64 length for the domain and every name/value,
    // followed by a little-endian u64 entry count and byte-sorted property names.
    let mut expected_bytes = Vec::new();
    expected_bytes.extend_from_slice(&20_u64.to_le_bytes());
    expected_bytes.extend_from_slice(b"property-environment");
    expected_bytes.extend_from_slice(&2_u64.to_le_bytes());
    expected_bytes.extend_from_slice(&14_u64.to_le_bytes());
    expected_bytes.extend_from_slice(b"PROPTEST_CASES");
    expected_bytes.extend_from_slice(&3_u64.to_le_bytes());
    expected_bytes.extend_from_slice(b"512");
    expected_bytes.extend_from_slice(&17_u64.to_le_bytes());
    expected_bytes.extend_from_slice(b"PROPTEST_RNG_SEED");
    expected_bytes.extend_from_slice(&2_u64.to_le_bytes());
    expected_bytes.extend_from_slice(b"99");
    assert_eq!(actual, digest(&expected_bytes));
}

#[cfg(unix)]
#[test]
fn non_utf8_unix_values_and_names_preserve_distinct_raw_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let first = OsString::from_vec(vec![b'v', 0xff]);
    let second = OsString::from_vec(vec![b'v', 0xfe]);
    assert_eq!(first.to_string_lossy(), second.to_string_lossy());
    assert_ne!(
        fingerprint([(OsString::from("PROPTEST_RNG_SEED"), first.clone())]),
        fingerprint([(OsString::from("PROPTEST_RNG_SEED"), second.clone())]),
    );
    let mut first_name = b"PROPTEST_FUTURE_".to_vec();
    first_name.push(0xff);
    let mut second_name = b"PROPTEST_FUTURE_".to_vec();
    second_name.push(0xfe);
    assert_ne!(
        fingerprint([(OsString::from_vec(first_name), OsString::from("same"))]),
        fingerprint([(OsString::from_vec(second_name), OsString::from("same"))]),
    );
}
