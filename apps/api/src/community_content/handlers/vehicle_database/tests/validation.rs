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
        id: Uuid::nil(),
        name: "BMP-2".into(),
        faction: "OPFOR".into(),
        armor_type: "MBT".into(),
        amphibious: "no".into(),
        primary_threat: String::new(),
        profile_image_url: "/uploads/bmp2.png".into(),
    }
}

#[test]
fn write_body_trims_every_field_and_stores_empty_optionals_as_none() {
    let fields = write_body(
        r#"{"name":"  M1A2 ","faction":" BLUFOR","armor_type":"MBT  ","amphibious":"   ","primary_threat":" ATGM ","profile_image_url":" https://example.com/m1.png "}"#,
    )
    .validate()
    .expect("valid body");
    assert_eq!(
        fields,
        VehicleFields {
            name: "M1A2".into(),
            faction: "BLUFOR".into(),
            armor_type: "MBT".into(),
            amphibious: None,
            primary_threat: Some("ATGM".into()),
            profile_image_url: Some("https://example.com/m1.png".into()),
        }
    );
}

#[test]
fn write_body_without_optional_keys_or_with_null_optionals_stores_none() {
    for json in [
        r#"{"name":"BTR-70","faction":"OPFOR","armor_type":"APC"}"#,
        r#"{"name":"BTR-70","faction":"OPFOR","armor_type":"APC","amphibious":null,"primary_threat":null,"profile_image_url":null}"#,
    ] {
        let fields = write_body(json).validate().expect("valid body");
        assert_eq!(fields.amphibious, None, "{json}");
        assert_eq!(fields.primary_threat, None, "{json}");
        assert_eq!(fields.profile_image_url, None, "{json}");
    }
}

#[test]
fn write_body_refuses_blank_required_fields() {
    for (json, key) in [
        (r#"{"name":"  ","faction":"F","armor_type":"A"}"#, "name"),
        (r#"{"name":"N","faction":"","armor_type":"A"}"#, "faction"),
        (
            r#"{"name":"N","faction":"F","armor_type":"\t\n"}"#,
            "armor_type",
        ),
        (
            r#"{"name":"\ufeff","faction":"F","armor_type":"A"}"#,
            "name",
        ),
    ] {
        let message = refusal(write_body(json).validate());
        assert!(message.starts_with(key), "{json}: {message}");
    }
}

#[test]
fn limits_count_trimmed_characters() {
    let at_limit = "é".repeat(120);
    let json = format!(r#"{{"name":"  {at_limit}  ","faction":"F","armor_type":"A"}}"#);
    let fields = write_body(&json).validate().expect("120 characters fit");
    assert_eq!(fields.name.chars().count(), 120);

    for (key, limit) in [
        ("name", 120),
        ("faction", 60),
        ("armor_type", 60),
        ("amphibious", 60),
        ("primary_threat", 120),
    ] {
        let mut body = serde_json::json!({"name": "N", "faction": "F", "armor_type": "A"});
        body[key] = serde_json::Value::String("x".repeat(limit + 1));
        let decoded: VehicleWriteBody = serde_json::from_value(body).expect("decodes");
        let message = refusal(decoded.validate());
        assert!(
            message.contains(&format!("{key} must be at most {limit}")),
            "{key}: {message}"
        );

        let mut patch = serde_json::Map::new();
        patch.insert(key.into(), serde_json::Value::String("x".repeat(limit + 1)));
        let decoded: VehiclePatchBody =
            serde_json::from_value(serde_json::Value::Object(patch)).expect("decodes");
        refusal(decoded.validate());
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
fn write_body_refuses_missing_or_null_required_fields() {
    for json in [
        r#"{"faction":"F","armor_type":"A"}"#,
        r#"{"name":null,"faction":"F","armor_type":"A"}"#,
        r#"{"name":"N","armor_type":"A"}"#,
        r#"{"name":"N","faction":"F"}"#,
    ] {
        assert!(
            serde_json::from_str::<VehicleWriteBody>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn patch_tells_absent_from_null_from_value() {
    let body = patch_body(r#"{"amphibious":null,"primary_threat":"RPG"}"#);
    assert_eq!(body.name, None);
    assert_eq!(body.amphibious, Some(None));
    assert_eq!(body.primary_threat, Some(Some("RPG".into())));
    assert_eq!(body.profile_image_url, None);
}

#[test]
fn patch_refuses_null_or_blank_required_fields() {
    for (json, key) in [
        (r#"{"name":null}"#, "name must not be null"),
        (r#"{"faction":null}"#, "faction must not be null"),
        (r#"{"armor_type":null}"#, "armor_type must not be null"),
        (r#"{"name":"   "}"#, "name must not be blank"),
        (r#"{"faction":""}"#, "faction must not be blank"),
    ] {
        let message = refusal(patch_body(json).validate());
        assert_eq!(message, key, "{json}");
    }
}

#[test]
fn empty_patch_changes_nothing() {
    let changes = patch_body("{}").validate().expect("empty patch is valid");
    assert!(changes.changed_keys().is_empty());
    let stored = stored_row();
    let fields = changes.applied_to(&stored);
    assert_eq!(
        fields,
        VehicleFields {
            name: "BMP-2".into(),
            faction: "OPFOR".into(),
            armor_type: "MBT".into(),
            amphibious: Some("no".into()),
            primary_threat: None,
            profile_image_url: Some("/uploads/bmp2.png".into()),
        }
    );
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

#[test]
fn patch_keeps_a_stored_value_that_predates_the_limits() {
    let mut stored = stored_row();
    stored.name = "n".repeat(300);
    stored.profile_image_url = "img".into();
    let fields = patch_body(r#"{"armor_type":"IFV"}"#)
        .validate()
        .expect("valid patch")
        .applied_to(&stored);
    assert_eq!(fields.name, stored.name);
    assert_eq!(fields.profile_image_url.as_deref(), Some("img"));
    assert_eq!(fields.armor_type, "IFV");
}
