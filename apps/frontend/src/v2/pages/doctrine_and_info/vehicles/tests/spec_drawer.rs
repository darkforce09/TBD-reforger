//! The dossier's image sanitising and amphibious chip.

use super::{amphib_badge, profile_image_src, BADGE_NEUTRAL, BADGE_SUCCESS, BADGE_WARNING};
use crate::v2::core::api::dto::vehicles::Vehicle;
use crate::v2::core::api::dto::DataEnvelope;

/// The recorded `GET /vehicle-database` answer the DOM oracle serves.
const RECORDED_LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/fixtures/api_goldens/GET__vehicle-database.json"
));

fn with_image(url: &str) -> Vehicle {
    Vehicle {
        id: "1".into(),
        name: "BTR-70".into(),
        faction: "USSR".into(),
        armor_type: "APC".into(),
        amphibious: String::new(),
        primary_threat: String::new(),
        profile_image_url: url.into(),
    }
}

#[test]
fn a_safe_image_url_loads_as_stored() {
    for url in [
        "https://cdn.tbd-reforger.example/iff/btr70.png",
        "/uploads/2d8f0c1e-7b1a-4c55-9a53-2b1f6d1a0c9e.webp",
    ] {
        assert_eq!(profile_image_src(&with_image(url)), Some(url), "{url}");
    }
}

#[test]
fn an_empty_or_unsafe_image_url_shows_the_placeholder() {
    for url in [
        "",
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "http://example.com/a.png",
        "//evil.example/a.png",
        "https:///a.png",
        "/",
        "img",
        " https://example.com/padded.png",
        "https://example.com/a b.png",
        "vbscript:msgbox(1)",
        "file:///etc/passwd",
    ] {
        assert_eq!(profile_image_src(&with_image(url)), None, "{url:?}");
    }
}

#[test]
fn every_recorded_image_is_safe_and_the_row_without_one_shows_the_placeholder() {
    let list: DataEnvelope<Vehicle> =
        serde_json::from_str(RECORDED_LIST).expect("the recorded list decodes");
    for vehicle in &list.data {
        let expected =
            (!vehicle.profile_image_url.is_empty()).then_some(vehicle.profile_image_url.as_str());
        assert_eq!(profile_image_src(vehicle), expected, "{}", vehicle.name);
    }
}

#[test]
fn the_amphibious_chip_warns_for_yes_and_clears_for_no() {
    for yes in ["Yes", " y ", "TRUE"] {
        assert_eq!(amphib_badge(yes), BADGE_WARNING, "{yes:?}");
    }
    for no in ["No", "n", "false"] {
        assert_eq!(amphib_badge(no), BADGE_SUCCESS, "{no:?}");
    }
    assert_eq!(amphib_badge("Limited"), BADGE_NEUTRAL);
}
