//! The two things the compile owes its callers besides the document: it refuses over-capacity
//! cargo with the same strings Save uses, and it hands the compile's structured findings out
//! ALONGSIDE the bytes rather than inside them.
//!
//! The mission fixture is shared with `mission_compile_flatten.rs`, which is where it is
//! predominantly used.

use website_map_engine::data::scenario::wire_safety::CargoPhys;

use super::flatten_tests::{FIXTURE, fixture_mission};
use super::*;
use crate::missions::contract::schema_validators::{
    validate_mission_document, validate_mission_editor_payload_with_catalog,
};

/// The same phys table + over-capacity numbers Save's cargo Class-R uses
/// (`missions::handlers::mission_versions` /
/// `missions::contract::schema_validators`), so the two boundaries are compared on one fixture.
fn cargo_phys_catalog_fixture() -> CargoPhysCatalog {
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
    catalog
}

/// One placed slot (so flatten would otherwise succeed) carrying cargo qty against a vest.
fn cargo_slot_payload(qty: u32) -> String {
    format!(
        r#"{{
          "schemaVersion": 1,
          "editor": {{
            "factions": [{{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}}],
            "squads": [{{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "slotIds": ["s1"]}}],
            "slots": [{{
              "id": "s1", "squadId": "sq1", "index": 0, "role": "RFL",
              "position": {{"x": 100.0, "y": 200.0, "z": 0, "rotation": 0}},
              "loadout": {{"version": 2,
                "wear": {{"vest": "vest_rn"}}, "weapons": [],
                "cargo": [{{"container": "vest", "item": "mag", "qty": {qty}}}]}}
            }}],
            "editorLayers": []
          }}
        }}"#
    )
}

/// With the same catalog Save uses, compile refuses the same over-capacity finding.
///
/// RED: delete the `scan_cargo_capacity` call from `flatten_to_mod_document_with_catalog`.
/// RED: invent a second cargo arithmetic that disagrees with Save's finding string.
#[test]
fn compile_with_catalog_refuses_over_capacity_like_save() {
    let m = fixture_mission();
    let catalog = cargo_phys_catalog_fixture();
    let bad = cargo_slot_payload(4);
    let ok = cargo_slot_payload(3);

    // Save channel (exact helper CreateVersion uses after loading the catalog) + the shared
    // scan itself — compile must refuse with the same strings, not a restated message.
    let save_details =
        validate_mission_editor_payload_with_catalog(bad.as_bytes(), &catalog).expect("schema");
    let parsed: serde_json::Value = serde_json::from_str(&bad).unwrap();
    let scan = wire_safety::scan_cargo_capacity(&parsed, &catalog);
    assert!(
        !scan.is_empty()
            && scan
                .iter()
                .any(|d| d.contains("240 / 200 cm³") && d.contains("Plate Carrier")),
        "shared scan must fire on this fixture: {scan:?}"
    );
    for d in &scan {
        assert!(
            save_details.iter().any(|s| s == d),
            "Save details must include scan finding {d:?}; got {save_details:?}"
        );
    }

    // Compile channel — same helper, same strings, Parse so `/compiled` cannot ship it.
    let err = flatten_to_mod_document_with_catalog(&m, bad.as_bytes(), &catalog)
        .expect_err("over-capacity must refuse at compile");
    let CompileError::Parse(detail) = err else {
        panic!("expected CompileError::Parse, got {err:?}");
    };
    for d in &scan {
        assert!(
            detail.contains(d.as_str()),
            "compile refuse missing scan finding {d:?}; got {detail}"
        );
    }

    flatten_to_mod_document_with_catalog(&m, ok.as_bytes(), &catalog)
        .expect("under-capacity must compile");
    flatten_to_mod_document_with_catalog(&m, bad.as_bytes(), &CargoPhysCatalog::new())
        .expect("empty catalog must not invent a limit (matches Save silence)");
}

/// Class-R — the no-arg compile entry always routes through the catalogued gate (empty catalog
/// today). RED: `flatten_to_mod_document` bypasses `with_catalog` again.
#[test]
fn flatten_routes_through_catalogued_compile_gate() {
    const SRC: &str = include_str!("../mission_compile.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("mission_compile.rs must declare its sibling test modules");
    assert!(
        production.contains("fn flatten_to_mod_document_with_catalog("),
        "catalogued compile gate must exist"
    );
    assert!(
        production.contains("wire_safety::scan_cargo_capacity"),
        "compile gate must call the same cargo helper Save uses"
    );
    let no_arg = production
        .split("pub fn flatten_to_mod_document(")
        .nth(1)
        .and_then(|s| {
            s.split("pub fn flatten_to_mod_document_with_catalog(")
                .next()
        })
        .expect("flatten_to_mod_document must exist before with_catalog");
    assert!(
        no_arg.contains("flatten_to_mod_document_with_catalog("),
        "no-arg flatten must delegate to the catalogued gate"
    );
}

/// Class-R — compile-as-trust-saved for the empty-catalog default: Save and live `/compiled` own
/// the refuse via `load_cargo_phys_catalog`. RED: Save drops the catalog load, or this adapter
/// stops documenting that dependency.
#[test]
fn compile_documents_save_cargo_refuse() {
    const COMPILE: &str = include_str!("../mission_compile.rs");
    let production = COMPILE
        .split("#[cfg(test)]")
        .next()
        .expect("mission_compile.rs must declare its sibling test modules");
    assert!(
        production.contains("load_cargo_phys_catalog"),
        "compile adapter must name Save's catalog loader (trust-saved contract)"
    );
    assert!(
        production.contains("validate_mission_editor_payload_with_catalog"),
        "compile adapter must name Save's catalogued validator"
    );
    assert!(
        production.contains("GET /missions/:id/compiled"),
        "compile adapter must name the live /compiled catalogued path"
    );

    const VERSIONS: &str = include_str!("../../handlers/mission_versions.rs");
    let versions_prod = VERSIONS
        .split("#[cfg(test)]")
        .next()
        .expect("mission_versions.rs must declare a sibling test module");
    assert!(
        versions_prod.contains("load_cargo_phys_catalog"),
        "Save must still load registry phys into the catalog"
    );
    let helper = versions_prod
        .split("fn validate_payload_with_catalog(")
        .nth(1)
        .expect("validate_payload_with_catalog must exist");
    assert!(
        helper.contains("validate_mission_editor_payload_with_catalog"),
        "Save helper must call the catalogued validator"
    );
    const HANDLER: &str = include_str!("../../handlers/mission_export.rs");
    let handler_prod = HANDLER
        .split("#[cfg(test)]")
        .next()
        .expect("mission_export.rs must declare a sibling test module");
    let compiled = handler_prod
        .split("pub async fn get_compiled_mission(")
        .nth(1)
        .and_then(|s| s.split("\nfn unreadable_stored_payload(").next())
        .expect("get_compiled_mission body");
    assert!(
        compiled.contains("load_cargo_phys_catalog")
            && compiled.contains("flatten_to_mod_document_with_catalog("),
        "/compiled must load catalog + with_catalog; got:\n{compiled}"
    );
}

/* ══════════ the compile's diagnostics reach this boundary ══════════ */

/// The header value: each rule id once, in first-fired order, comma-separated. Empty → `None`,
/// so a clean compile omits the header instead of sending a blank one.
#[test]
fn the_rules_header_dedupes_and_keeps_fire_order() {
    let f = |rule_id: &'static str| CompileFinding {
        rule_id,
        severity: FindingSeverity::Info,
        primitive: website_map_engine::data::scenario::validate::Primitive::PerObjectInvariant,
        message: String::new(),
        subject: "/editor/slots/0".into(),
        subject_id: None,
    };
    assert_eq!(compile_diagnostics_rules_header(&[]), None);
    assert_eq!(
        compile_diagnostics_rules_header(&[f("B-RULE"), f("A-RULE"), f("B-RULE")]).as_deref(),
        Some("B-RULE,A-RULE"),
        "fire order, not sorted — the compile's walk order is the meaningful one"
    );
}

/// The clean-input rule at THIS boundary: the shipped fixture — a real two-faction mission with
/// loadouts — compiles with an empty finding list, and the count header would read `0`.
#[test]
fn the_fixture_mission_compiles_with_no_diagnostics() {
    let doc = flatten_to_mod_document(&fixture_mission(), FIXTURE.as_bytes()).expect("compiles");
    assert!(
        doc.diagnostics.is_empty(),
        "a clean mission must produce no findings at the /compiled boundary; got {:?}",
        doc.diagnostics
    );
    assert_eq!(compile_diagnostics_rules_header(&doc.diagnostics), None);
}

/// A payload that authors a dropped value still serves a VALID document — findings ride
/// alongside the bytes, never inside them. Without this, the obvious "just add a `diagnostics` key"
/// implementation would 500 `/compiled` for every mission (`additionalProperties: false` on the
/// document root).
#[test]
fn a_mission_with_findings_still_serves_a_schema_valid_document() {
    // The seed authors a rank that is genuinely off the schema's ladder — the enum gate drops it,
    // one finding — while `stance` stays a representable value and EMITS. That keeps this
    // boundary's subject intact (a document with findings is still served, and still valid) and
    // adds the other half: the emitted keys must not break it.
    //
    // A seed that pins a LOSS has to be re-aimed the moment the loss is repaired, or it argues for
    // keeping the gap: both `rank` and `stance` now reach the wire when representable, so only the
    // off-ladder rank can still be dropped.
    let payload = FIXTURE.replace(
        r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL","#,
        r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "rank": "Lance Corporal", "stance": "prone","#,
    );
    assert_ne!(payload, FIXTURE, "the seed must change the fixture");
    let doc =
        flatten_to_mod_document(&fixture_mission(), payload.as_bytes()).expect("still compiles");
    assert_eq!(
        doc.diagnostics.len(),
        1,
        "the off-ladder rank is dropped and reported; the representable stance is not; got {:?}",
        doc.diagnostics
    );
    assert_eq!(
        compile_diagnostics_rules_header(&doc.diagnostics).as_deref(),
        Some("COMPILE-DROP-SLOT-RANK")
    );

    let body = serde_json::to_vec(&doc).expect("serialises");
    assert!(
        validate_mission_document(&body)
            .expect("validator available")
            .is_empty(),
        "the served document must stay schema-valid with findings present"
    );
    assert!(
        !String::from_utf8_lossy(&body).contains("diagnostics"),
        "the findings must never reach the wire body"
    );
}
