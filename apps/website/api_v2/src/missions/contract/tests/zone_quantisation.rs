use super::*;
use crate::missions::contract::schema_validators::validate_mission_editor_payload;

/// A payload carrying one zone, wrapped so it also satisfies the rest of the editor schema.
fn payload_with_zones(zones: &str) -> Vec<u8> {
    format!(
        r#"{{"schemaVersion":1,"zones":{zones},
                "editor":{{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}}}"#
    )
    .into_bytes()
}

/// **DEFECT 1** — the executed repro, as a save-time finding.
///
/// RED without `scan_authored_zones`: `details` is empty, `create_version` answers 201, and
/// `GET /compiled` answers 500 forever because a `mission_versions` row is immutable.
#[test]
fn undeclared_zone_rule_key_is_a_save_time_finding() {
    let bad = payload_with_zones(
        r#"[{"id":"z1","type":"objective_capture",
                 "shape":{"circle":{"x":100,"z":200,"r":50}},
                 "rules":{"notInVocabulary":1}}]"#,
    );
    let details = validate_mission_editor_payload(&bad).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(details[0].starts_with("/zones/0/rules:"), "{details:?}");
    assert!(details[0].contains("notInVocabulary"), "{details:?}");

    // A rule key that IS in the declared vocabulary still saves clean.
    let ok = payload_with_zones(
        r#"[{"id":"z1","type":"objective_capture",
                 "shape":{"circle":{"x":100,"z":200,"r":50}},
                 "rules":{"captureSeconds":90}}]"#,
    );
    assert!(
        validate_mission_editor_payload(&ok)
            .expect("compiles")
            .is_empty(),
        "a declared rule must still be accepted"
    );
}

/// **DEFECT 1, second mechanism** — `type` outside the six-value enum, same 201-then-500.
#[test]
fn zone_type_outside_the_enum_is_a_save_time_finding() {
    let bad = payload_with_zones(
        r#"[{"id":"z1","type":"capture","shape":{"circle":{"x":1,"z":2,"r":50}}}]"#,
    );
    let details = validate_mission_editor_payload(&bad).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(details[0].starts_with("/zones/0/type:"), "{details:?}");

    for kind in [
        "spawn",
        "objective_capture",
        "objective_destroy",
        "objective_hold_until",
        "boundary",
        "base_protection",
    ] {
        let ok = payload_with_zones(&format!(
            r#"[{{"id":"z1","type":"{kind}","shape":{{"circle":{{"x":1,"z":2,"r":50}}}}}}]"#
        ));
        assert!(
            validate_mission_editor_payload(&ok)
                .expect("compiles")
                .is_empty(),
            "the declared type {kind} must still be accepted"
        );
    }
}

/// **DEFECT 2 — the post-quantisation case, and the reason this check cannot be a rule on the
/// authored document.**
///
/// `r: 0.04` satisfies `$defs/circle.r` `exclusiveMinimum: 0` exactly as written, and
/// `shape_from_input`'s own `r > 0.0` gate passes it. Only `round_coord` makes it `0.0`, and
/// only the EMITTED row shows that. A click without a drag in the draw tool produces this
/// radius. RED if `projected_shape` is changed to carry the authored radius through unrounded.
#[test]
fn click_without_drag_radius_is_caught_after_quantisation() {
    let bad = payload_with_zones(
        r#"[{"id":"z1","type":"boundary","shape":{"circle":{"x":100,"z":200,"r":0.04}}}]"#,
    );
    let details = validate_mission_editor_payload(&bad).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(details[0].starts_with("/zones/0/shape:"), "{details:?}");
    // The author typed 0.04; the raw schema finding quotes an r of 0.0 they never wrote.
    assert!(details[0].contains("0.04"), "{details:?}");
    assert!(details[0].contains("0.05 m"), "{details:?}");

    // 0.05 is the first radius that survives the 0.1 m grid — it must still be accepted.
    let ok = payload_with_zones(
        r#"[{"id":"z1","type":"boundary","shape":{"circle":{"x":100,"z":200,"r":0.05}}}]"#,
    );
    assert!(
        validate_mission_editor_payload(&ok)
            .expect("compiles")
            .is_empty(),
        "the smallest radius that survives quantisation must be accepted"
    );
}

/// The invariant this boundary holds: the accept set at save must not be narrower than the
/// compile set. `flatten_authored_zone` DROPS each of these before the document exists, so none
/// can reach a game server and none may be refused here.
#[test]
fn zones_the_compile_drops_are_not_refused() {
    for dropped in [
        r#"[{}]"#,                                                              // nothing at all
        r#"[{"id":"z1","type":"boundary"}]"#,                                   // no shape
        r#"[{"id":"","type":"boundary","shape":{"circle":{"r":9}}}]"#,          // empty id
        r#"[{"id":"z1","type":"","shape":{"circle":{"r":9}}}]"#,                // empty type
        r#"[{"id":"z1","type":"boundary","shape":{"circle":{"r":0}}}]"#,        // r not > 0
        r#"[{"id":"z1","type":"boundary","shape":{"polygon":[[1,2],[3,4]]}}]"#, // < 3 vertices
        r#"[]"#,                                                                // no zones
    ] {
        let details =
            validate_mission_editor_payload(&payload_with_zones(dropped)).expect("compiles");
        assert!(
            details.is_empty(),
            "a zone the compile drops must not be refused at save: {dropped} -> {details:?}"
        );
    }
}

/// A polygon zone round-trips, and its vertices are quantised the same way.
#[test]
fn polygon_zone_is_accepted_and_quantised() {
    let ok = payload_with_zones(
        r#"[{"id":"z_bounds","type":"boundary","label":"Play area",
                 "faction":"BLUFOR",
                 "shape":{"polygon":[[0,0],[1000.256,0],[1000.256,175.789],[0,175.789]]}}]"#,
    );
    assert!(
        validate_mission_editor_payload(&ok)
            .expect("compiles")
            .is_empty(),
        "an uppercase authored faction must NOT be validated against $defs/factionKey"
    );
}

/// The quantisation this file mirrors is `flatten::round_coord`, which is private there.
/// Pin it against that source so the mirror cannot drift silently — silent drift between two
/// sites is the real bug class. RED if `flatten.rs` changes its grid without this file following.
#[test]
fn zone_quantisation_mirrors_flatten() {
    let flatten =
        include_str!("../../../../../map-engine/src/data/scenario/compiler/flatten/zones.rs");
    let body = flatten
        .split("fn round_coord(v: f64) -> f64 {")
        .nth(1)
        .expect("flatten::round_coord must exist");
    let expr = body.split('}').next().expect("body").trim();
    assert_eq!(
        expr, "(v * 10.0).round() / 10.0",
        "flatten::round_coord changed — update contract::zone_quantisation::round_coord to match"
    );
    // And the mirror agrees on the value that produced the defect.
    assert_eq!(round_coord(0.04), 0.0);
    assert_eq!(round_coord(0.05), 0.1);
    assert_eq!(round_coord(1000.256), 1000.3);
}

/// The verdict must come from `mission.schema.json` itself, not from a copy of it. If the
/// wrapper stopped resolving `#/$defs/zone`, every zone would validate against nothing and
/// this whole pass would go silently vacuous — the signature defect.
#[test]
fn zone_schema_resolves_the_real_defs() {
    let v = compile_zone_schema().expect("wrapper must compile");
    assert!(
        v.is_valid(&json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":5}}})),
        "a good zone must validate"
    );
    // Each of these can only fail if a DIFFERENT $def was reached: zone → shape → circle,
    // zone → zoneRules.
    assert!(!v.is_valid(&json!({"id":"z1","type":"nope","shape":{"circle":{"x":0,"z":0,"r":5}}})));
    assert!(
        !v.is_valid(&json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":0}}}))
    );
    assert!(!v.is_valid(
        &json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":5}},
                    "rules":{"graceSeconds":-1}})
    ));
}
