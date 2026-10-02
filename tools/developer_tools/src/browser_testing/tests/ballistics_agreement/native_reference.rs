//! Unit tests for the native half of the ballistics agreement gate; the shared mapping, lead
//! summary and bit walk are tested in the map engine.

use super::*;
use crate::browser_testing::ballistics_agreement::COMMITTED_CATALOG;
use crate::browser_testing::server::repo_root;

const SEED: u64 = 0x5EED_0000_0000_0002;

fn committed_catalog() -> BallisticsCatalog {
    let path = repo_root().join(COMMITTED_CATALOG);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "the committed catalog {} is unreadable: {error}",
            path.display()
        )
    });
    BallisticsCatalog::from_json_slice(&bytes).expect("the committed catalog decodes")
}

#[test]
fn the_native_cases_are_the_drawn_cases_in_order() {
    let catalog = committed_catalog();
    let native = native_cases(&catalog, SEED, 3);
    let drawn = agreement_cases(&catalog, SEED, 3);
    assert_eq!(native.len(), 3);
    for (native_case, drawn_case) in native.iter().zip(&drawn) {
        assert_eq!(native_case.case_id, drawn_case.case_id);
        assert_eq!(
            native_case.inputs,
            fire_mission_inputs(&catalog, drawn_case)
        );
        assert_eq!(
            native_case.bit_patterns,
            case_bit_patterns(&native_case.inputs, native_case.solution())
        );
        assert_eq!(
            native_case.solution().is_some(),
            native_case.refusal().is_none()
        );
    }
}

#[test]
fn every_f64_is_keyed_by_its_pointer_and_integers_are_not() {
    let catalog = committed_catalog();
    let case = &native_cases(&catalog, SEED, 1)[0];
    let solution = case.solution().expect("the case solves");
    assert_eq!(
        case.bit_patterns.get("/solution/guns/0/azimuth_mils"),
        Some(&format!("{:016x}", solution.guns[0].azimuth_mils.to_bits()))
    );
    assert!(case.bit_patterns.contains_key("/inputs/target/x"));
    assert!(!case.bit_patterns.contains_key("/inputs/catalog_version"));
    assert!(
        !case
            .bit_patterns
            .contains_key("/solution/guns/0/mils_per_circle")
    );
}
