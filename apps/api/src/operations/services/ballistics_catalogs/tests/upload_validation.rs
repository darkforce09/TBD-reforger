use super::*;

#[test]
fn game_ballistics_slug_pattern_matches_the_contract() {
    for good in [
        "vanilla_mortars",
        "a",
        "m252-81mm",
        "a1_b2-c3",
        &"a".repeat(64),
    ] {
        assert!(is_slug(good), "{good} is a slug");
    }
    for bad in [
        "",
        "Vanilla",
        "_lead",
        "trail-",
        "double__joint",
        "space here",
        "dot.ted",
        &"a".repeat(65),
    ] {
        assert!(!is_slug(bad), "{bad:?} is not a slug");
    }
}

#[test]
fn game_ballistics_game_build_pattern_matches_the_contract() {
    for good in ["1.8", "1.8.0", "1.8.0.13"] {
        assert!(is_game_build(good), "{good} is a game build");
    }
    for bad in ["", "1", "1.8.0.13.2", "1..8", "1.8a", ".1.8", "v1.8"] {
        assert!(!is_game_build(bad), "{bad:?} is not a game build");
    }
}

#[test]
fn game_ballistics_enfusion_guid_pattern_matches_the_contract() {
    assert!(is_enfusion_guid("6A6F008DC5395616"));
    for bad in [
        "6a6f008dc5395616",
        "6A6F008DC539561",
        "6A6F008DC53956167",
        "6A6F008DC539561G",
    ] {
        assert!(!is_enfusion_guid(bad), "{bad:?} is not a GUID");
    }
}

#[test]
fn game_ballistics_undecodable_parts_are_refused_by_part() {
    let refusal = decode_upload(b"not json", b"{}").unwrap_err();
    assert_eq!(refusal.code(), "catalog_undecodable");
    assert!(
        refusal
            .message()
            .starts_with("the catalog part does not decode")
    );
}

#[test]
fn game_ballistics_upload_report_accepts_exactly_without_failures() {
    let report = CatalogUploadReport::from_calibration(CalibrationReport::default());
    assert!(report.accepted);
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["accepted"], serde_json::json!(true));
    assert_eq!(value["cases"], serde_json::json!(0));
    assert_eq!(value["failures"], serde_json::json!([]));
    assert_eq!(value["forward_samples_not_judged"], serde_json::json!(0));
}
