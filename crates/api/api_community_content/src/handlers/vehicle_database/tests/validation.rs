use axum::http::StatusCode;
use uuid::Uuid;

use super::*;

fn write_body(json: &str) -> VehicleWriteBody {
    serde_json::from_str(json).expect("write body decodes")
}

fn patch_body(json: &str) -> VehiclePatchBody {
    serde_json::from_str(json).expect("patch body decodes")
}

fn refusal(result: Result<impl std::fmt::Debug, ApiError>) -> String {
    let error = result.expect_err("the body must be refused");
    assert_eq!(error.status, StatusCode::BAD_REQUEST, "{}", error.message);
    error.message
}

fn stored_row() -> VehicleDatabase {
    VehicleDatabase {
        id: Uuid::nil().into(),
        name: "BMP-2".into(),
        faction: "OPFOR".into(),
        armor_type: "MBT".into(),
        amphibious: "no".into(),
        primary_threat: String::new(),
        profile_image_url: "/uploads/bmp2.png".into(),
    }
}

#[test]
fn profile_image_url_accepts_https_and_site_paths_and_refuses_everything_else() {
    for url in ["https://example.com/a.png", "/uploads/a.webp", ""] {
        let json =
            format!(r#"{{"name":"N","faction":"F","armor_type":"A","profile_image_url":"{url}"}}"#);
        write_body(&json).validate().expect(url);
    }
    for url in [
        "http://example.com/a.png",
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "//evil.example/a.png",
        "https:///a.png",
        "/",
        "uploads/a.png",
        "https://example.com/a b.png",
    ] {
        let body = serde_json::json!({
            "name": "N", "faction": "F", "armor_type": "A", "profile_image_url": url
        });
        let decoded: VehicleWriteBody = serde_json::from_value(body).expect("decodes");
        let message = refusal(decoded.validate());
        assert!(message.starts_with("profile_image_url"), "{url}: {message}");
    }
}

#[test]
fn both_bodies_refuse_unknown_keys() {
    assert!(
        serde_json::from_str::<VehicleWriteBody>(
            r#"{"name":"N","faction":"F","armor_type":"A","crew":3}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<VehiclePatchBody>(r#"{"crew":3}"#).is_err());
    assert!(serde_json::from_str::<VehiclePatchBody>(r#"{"id":"x"}"#).is_err());
}

#[test]
fn patch_applies_values_clears_optionals_and_keeps_absent_fields() {
    let changes = patch_body(
        r#"{"faction":" Independent ","amphibious":null,"primary_threat":"  RPG ","profile_image_url":""}"#,
    )
    .validate()
    .expect("valid patch");
    assert_eq!(
        changes.changed_keys(),
        vec![
            "faction",
            "amphibious",
            "primary_threat",
            "profile_image_url"
        ]
    );
    let fields = changes.applied_to(&stored_row());
    assert_eq!(
        fields,
        VehicleFields {
            name: "BMP-2".into(),
            faction: "Independent".into(),
            armor_type: "MBT".into(),
            amphibious: None,
            primary_threat: Some("RPG".into()),
            profile_image_url: None,
        }
    );
}
