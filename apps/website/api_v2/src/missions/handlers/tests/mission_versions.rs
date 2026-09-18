//! Save-boundary tests for the version handlers: the cargo-capacity refusal the catalogued
//! validator performs, plus source pins for the guards that run before an INSERT can land.

use super::*;

const VERSIONS: &str = include_str!("../mission_versions.rs");
const LIFECYCLE: &str = include_str!("../mission_lifecycle.rs");

/// The production half of a handler file — everything before its sibling-test declaration.
fn production_half<'a>(source: &'a str, file: &str) -> &'a str {
    source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_else(|| panic!("{file} must have a #[cfg(test)] module"))
}

/// Save refuses over-capacity cargo when the caller supplies phys attrs; an empty catalog stays
/// silent (never invent a limit). The numbers mirror the editor's own check: 4×60 cm³ into a
/// 200 cm³ vest.
#[test]
fn over_capacity_cargo_is_refused_at_save_with_catalog() {
    let mut catalog = CargoPhysCatalog::new();
    catalog.insert(
        "mag".into(),
        CargoPhys {
            display_name: "Mag".into(),
            weight_kg: Some(0.5),
            volume_cm3: Some(60.0),
            ..CargoPhys::default()
        },
    );
    catalog.insert(
        "vest_rn".into(),
        CargoPhys {
            display_name: "Plate Carrier".into(),
            max_weight_kg: Some(5.0),
            max_volume_cm3: Some(200.0),
            ..CargoPhys::default()
        },
    );
    let bad = r#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":4}]}}]}}"#;
    let err = validate_payload_with_catalog(bad, &catalog).expect_err("must refuse");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    let details = err
        .details
        .as_ref()
        .and_then(|d| d.as_array())
        .expect("details");
    assert!(
        details.iter().any(|d| {
            d.as_str()
                .is_some_and(|s| s.contains("240 / 200 cm³") && s.contains("Plate Carrier"))
        }),
        "Save details must name the over-capacity finding: {details:?}"
    );

    let ok = r#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":3}]}}]}}"#;
    assert!(
        validate_payload_with_catalog(ok, &catalog).is_ok(),
        "under-capacity must save"
    );
    assert!(
        validate_payload_with_catalog(bad, &CargoPhysCatalog::new()).is_ok(),
        "empty catalog must not invent a limit"
    );
}

/// The live Save boundary must call the catalogued validator, not the empty-catalog shortcut.
///
/// RED: swap `validate_mission_editor_payload_with_catalog` back to
/// `validate_mission_editor_payload` in `validate_payload_with_catalog`.
#[test]
fn save_payload_validation_uses_the_cargo_catalog_entry() {
    let production = production_half(VERSIONS, "mission_versions.rs");
    assert!(
        production.contains("load_cargo_phys_catalog"),
        "Save must load registry phys into the catalog"
    );
    let helper = production
        .split("fn validate_payload_with_catalog(")
        .nth(1)
        .expect("validate_payload_with_catalog must exist");
    assert!(
        helper.contains("validate_mission_editor_payload_with_catalog"),
        "Save helper must call the catalogued validator"
    );
    let stripped = helper.replace("validate_mission_editor_payload_with_catalog", "");
    assert!(
        !stripped.contains("validate_mission_editor_payload("),
        "validate_payload_with_catalog must not fall back to the empty-catalog entry"
    );
}

/// `create_version` must parse the semver before the INSERT — a trim-only guard lets a padded
/// duplicate land as a distinct unique key.
#[test]
fn create_version_guards_semver() {
    let production = production_half(VERSIONS, "mission_versions.rs");
    let body = production
        .split("pub async fn create_version(")
        .nth(1)
        .and_then(|s| s.split("pub async fn get_version(").next())
        .expect("create_version body");
    assert!(
        body.contains("valid_semver(&input.semver)"),
        "create_version must parse semver via valid_semver before INSERT"
    );
}

/// `create_version` must mirror a non-blank payload `title` onto `missions.title`, or the next
/// hydrate serves row meta that disagrees with the authored document.
///
/// RED: drop `payload_title_for_row_mirror` / the `title = $3` UPDATE arm — this pin fails.
#[test]
fn create_version_mirrors_payload_title_onto_row() {
    let production = production_half(VERSIONS, "mission_versions.rs");
    let create_version = production
        .split("pub async fn create_version(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("create_version body");
    assert!(
        create_version.contains("payload_title_for_row_mirror"),
        "create_version must extract payload title via payload_title_for_row_mirror"
    );
    assert!(
        create_version.contains("title = $3"),
        "create_version must UPDATE missions.title when mirroring; got:\n{create_version}"
    );
    const VERSION_PAYLOAD: &str = include_str!("../../validation/version_payload.rs");
    assert!(
        VERSION_PAYLOAD.contains("fn payload_title_for_row_mirror"),
        "payload_title_for_row_mirror helper must exist"
    );
    assert!(
        VERSION_PAYLOAD.contains("validated_mission_title(raw)"),
        "row-mirror must reuse validated_mission_title (non-blank trim guard)"
    );
}

/// `create_version` must call `reject_vacuous_version_payload` after `validate_payload` and
/// before the INSERT / `current_version_id` UPDATE. The gate is `create_version`-only:
/// `create_mission` still stores `{}` as its 0.1.0 stub.
///
/// RED: delete the `reject_vacuous_version_payload(payload_str)?` call — this pin fails.
#[test]
fn create_version_rejects_vacuous_payload_before_insert() {
    let production = production_half(VERSIONS, "mission_versions.rs");
    let body = production
        .split("pub async fn create_version(")
        .nth(1)
        .and_then(|s| s.split("pub async fn get_version(").next())
        .expect("create_version body");
    assert!(
        body.contains("reject_vacuous_version_payload(payload_str)"),
        "create_version must refuse vacuous payloads before INSERT"
    );
    let validate_at = body
        .find("validate_payload(&state.pool, payload_str)")
        .expect("create_version must still validate_payload");
    let reject_at = body
        .find("reject_vacuous_version_payload(payload_str)")
        .expect("reject call");
    assert!(
        reject_at > validate_at,
        "vacuous gate must run after validate_payload (schema errors first)"
    );
    assert!(
        body.contains("current_version_id = $1"),
        "create_version must still update current_version_id on success"
    );
    let create = production_half(LIFECYCLE, "mission_lifecycle.rs")
        .split("pub async fn create_mission(")
        .nth(1)
        .and_then(|s| s.split("pub async fn").next())
        .expect("create_mission body");
    assert!(
        !create.contains("reject_vacuous_version_payload"),
        "create_mission must keep allowing the empty stub payload"
    );
}

/// `set_current_version` must gate on `can_edit`, prove the target version belongs to the mission
/// (`mission_id = $2`), and UPDATE `current_version_id` (+ `updated_at`).
///
/// RED: drop `can_edit` / the `AND mission_id` predicate / the UPDATE — this pin fails.
#[test]
fn set_current_version_repaints_tip_with_belonging_check() {
    let production = production_half(VERSIONS, "mission_versions.rs");
    let body = production
        .split("pub async fn set_current_version(")
        .nth(1)
        .and_then(|s| {
            s.split("pub(crate) async fn load_cargo_phys_catalog(")
                .next()
        })
        .expect("set_current_version body");
    assert!(
        body.contains("can_edit(user, &m)"),
        "set_current_version must require can_edit (author or admin)"
    );
    assert!(
        body.contains("WHERE id = $1 AND mission_id = $2"),
        "set_current_version must refuse versions that do not belong to the mission"
    );
    assert!(
        body.contains("current_version_id = $1") && body.contains("updated_at = now()"),
        "set_current_version must UPDATE current_version_id and bump updated_at"
    );
    assert!(
        body.contains("mission.version.set_current"),
        "set_current_version must write an audit action distinct from create_version"
    );
    // Route registration lives in the missions route table, which `core::http_router` merges
    // under `/api/v1` — pin the path string here so a handler without a route cannot green.
    const ROUTES: &str = include_str!("../../routes.rs");
    assert!(
        ROUTES.contains("/missions/{id}/versions/{vid}/set-current")
            && ROUTES.contains("set_current_version"),
        "missions/routes.rs must register POST …/versions/{{vid}}/set-current → set_current_version"
    );
}
