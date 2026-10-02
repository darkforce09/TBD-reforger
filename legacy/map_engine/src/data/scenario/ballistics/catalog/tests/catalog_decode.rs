//! Tests of catalog decoding and its parity with `ballistics-catalog.schema.json`.

use serde_json::{Value, json};

use super::*;

/// The hand-written catalog every catalog test reads.
const MINIMAL_CATALOG_JSON: &str = include_str!("minimal_catalog.json");

/// The contract the catalog types project; a missing file fails the build, not the test.
const CATALOG_SCHEMA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/definitions/ballistics-catalog.schema.json"
));

fn sample_value() -> Value {
    serde_json::from_str(MINIMAL_CATALOG_JSON).expect("the sample catalog is JSON")
}

fn schema_validator() -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(CATALOG_SCHEMA_JSON).expect("the catalog schema is JSON");
    jsonschema::validator_for(&schema).expect("the catalog schema compiles")
}

fn schema_errors(validator: &jsonschema::Validator, document: &Value) -> Vec<String> {
    validator
        .iter_errors(document)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

fn decode(document: &Value) -> Result<BallisticsCatalog, CatalogDecodeError> {
    BallisticsCatalog::from_json_slice(&serde_json::to_vec(document).expect("value serialises"))
}

#[test]
fn sample_catalog_decodes_every_field() {
    let catalog = BallisticsCatalog::from_json_slice(MINIMAL_CATALOG_JSON.as_bytes())
        .expect("the sample catalog decodes");
    assert_eq!(catalog.schema_version, CATALOG_SCHEMA_VERSION);
    assert_eq!(catalog.catalog_id, "test-mortars");
    assert_eq!(catalog.catalog_version, 3);
    assert_eq!(catalog.game_build, "1.8.0.13");
    assert_eq!(catalog.gravity_m_s2, 9.81);
    assert_eq!(catalog.gravity_source, GravitySource::Oracle);
    assert_eq!(catalog.resources.len(), 1);
    assert_eq!(catalog.weapons.len(), 2);
    assert_eq!(catalog.shells.len(), 3);

    let weapon = &catalog.weapons[1];
    assert_eq!(weapon.weapon_id, "2b14");
    assert_eq!(weapon.mils_per_circle, 6000);
    assert_eq!(weapon.muzzle_init_speed_coef, 0.95);
    assert_eq!(weapon.shell_ids, ["o-832-he"]);

    let illumination = &catalog.shells[1];
    assert_eq!(illumination.role, ShellRole::Illumination);
    assert_eq!(illumination.side_air_drag_scale, 1.0);
    assert_eq!(
        illumination.time_fuze,
        Some(TimeFuze {
            min_s: 4.0,
            max_s: 50.0,
            default_s: 20.0
        })
    );
    assert_eq!(
        illumination.default_charge(),
        Some(&Charge {
            rings: 1,
            init_speed_coef: 1.55,
            is_default: true
        })
    );
    assert_eq!(catalog.shells[0].time_fuze, None);
    assert_eq!(
        catalog.shells[2].charge(4).map(|c| c.init_speed_coef),
        Some(2.5)
    );
    assert_eq!(catalog.shells[2].charge(3), None);
}

#[test]
fn sample_catalog_conforms_to_the_schema() {
    let errors = schema_errors(&schema_validator(), &sample_value());
    assert!(errors.is_empty(), "schema violations: {errors:#?}");
}

#[test]
fn re_encoded_catalog_conforms_to_the_schema_and_round_trips() {
    let catalog = decode(&sample_value()).expect("the sample catalog decodes");
    let encoded = serde_json::to_value(&catalog).expect("the catalog encodes");
    let errors = schema_errors(&schema_validator(), &encoded);
    assert!(errors.is_empty(), "schema violations: {errors:#?}");
    assert_eq!(encoded, sample_value());
    assert_eq!(decode(&encoded).expect("re-decodes"), catalog);
}

#[test]
fn unknown_fields_are_refused_by_decode_and_schema_alike() {
    let validator = schema_validator();
    let pointers = [
        "",
        "/resources/0",
        "/weapons/0",
        "/shells/0",
        "/shells/0/charges/0",
    ];
    for pointer in pointers.into_iter().chain(["/shells/1/time_fuze"]) {
        let mut document = sample_value();
        document
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("{pointer} is an object in the sample"))
            .insert("unexpected_field".to_owned(), json!(1));
        assert!(
            matches!(decode(&document), Err(CatalogDecodeError::Json(_))),
            "decode accepted an unknown field at `{pointer}`"
        );
        assert!(
            !schema_errors(&validator, &document).is_empty(),
            "schema accepted an unknown field at `{pointer}`"
        );
    }
}

#[test]
fn missing_required_fields_are_refused_by_decode_and_schema_alike() {
    let validator = schema_validator();
    let cases = [
        ("", "gravity_source"),
        ("/weapons/0", "mils_per_circle"),
        ("/shells/0", "side_air_drag_scale"),
        ("/shells/0/charges/0", "is_default"),
        ("/shells/1/time_fuze", "default_s"),
    ];
    for (pointer, field) in cases {
        let mut document = sample_value();
        document
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("{pointer} is an object in the sample"))
            .remove(field)
            .unwrap_or_else(|| panic!("{pointer}/{field} is present in the sample"));
        assert!(
            matches!(decode(&document), Err(CatalogDecodeError::Json(_))),
            "decode accepted a missing `{pointer}/{field}`"
        );
        assert!(
            !schema_errors(&validator, &document).is_empty(),
            "schema accepted a missing `{pointer}/{field}`"
        );
    }
}

#[test]
fn unknown_enum_values_are_refused_by_decode_and_schema_alike() {
    let validator = schema_validator();
    for (pointer, value) in [
        ("/gravity_source", "measured"),
        ("/shells/0/role", "cluster"),
    ] {
        let mut document = sample_value();
        *document.pointer_mut(pointer).expect("pointer exists") = json!(value);
        assert!(
            matches!(decode(&document), Err(CatalogDecodeError::Json(_))),
            "decode accepted `{value}` at `{pointer}`"
        );
        assert!(
            !schema_errors(&validator, &document).is_empty(),
            "schema accepted `{value}` at `{pointer}`"
        );
    }
}

#[test]
fn other_schema_versions_are_refused() {
    let mut document = sample_value();
    document["schema_version"] = json!(2);
    assert!(matches!(
        decode(&document),
        Err(CatalogDecodeError::UnsupportedSchemaVersion(2))
    ));
    assert!(!schema_errors(&schema_validator(), &document).is_empty());
}

#[test]
fn malformed_json_is_refused() {
    assert!(matches!(
        BallisticsCatalog::from_json_slice(b"{\"schema_version\": 1,"),
        Err(CatalogDecodeError::Json(_))
    ));
}
