//! Unit tests for the ballistics agreement bench's reading, over the committed vanilla catalog; the
//! shared mapping, lead summary and bit walk are tested in `ballistics_agreement_cases`.

use super::*;
use ballistics_agreement_cases::agreement_cases;

/// The committed catalog the agreement gate solves natively.
const COMMITTED_CATALOG: &str = "contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json";
const SEED: u64 = 0x5EED_0000_0000_0001;

fn committed_catalog() -> BallisticsCatalog {
    let path = frontend_test_support::repository_root::repository_path(
        env!("CARGO_MANIFEST_DIR"),
        COMMITTED_CATALOG,
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "the committed catalog {} is unreadable: {error}",
            path.display()
        )
    });
    BallisticsCatalog::from_json_slice(&bytes).expect("the committed catalog decodes")
}

/// Draws `count` cases over `catalog` from `seed` and solves every one, as the browser half does.
fn agreement_report(catalog: &BallisticsCatalog, seed: u64, count: usize) -> AgreementReport {
    let cases = agreement_cases(catalog, seed, count)
        .iter()
        .map(|case| case_report(catalog, case))
        .collect();
    assemble_report(catalog, seed, count, cases)
}

/// The drawn case `case_id` names among the first four cases of [`SEED`].
fn drawn_case(catalog: &BallisticsCatalog, case_id: &str) -> AgreementCase {
    agreement_cases(catalog, SEED, 4)
        .into_iter()
        .find(|case| case.case_id == case_id)
        .unwrap_or_else(|| panic!("`{case_id}` is a drawn case"))
}

#[test]
fn the_report_solves_every_drawn_case_in_order() {
    let catalog = committed_catalog();
    let report = agreement_report(&catalog, SEED, 4);
    assert_eq!(report.seed, SEED);
    assert_eq!(report.count, 4);
    assert_eq!(catalog.catalog_id, report.catalog_id);
    assert_eq!(report.catalog_version, catalog.catalog_version);
    assert_eq!(report.solver_revision, SOLVER_REVISION);
    let drawn: Vec<String> = agreement_cases(&catalog, SEED, 4)
        .into_iter()
        .map(|case| case.case_id.into_inner())
        .collect();
    let reported: Vec<String> = report
        .cases
        .iter()
        .map(|case| case.case_id.clone().into_inner())
        .collect();
    assert_eq!(reported, drawn);
    for case in &report.cases {
        let solution = case.solution.as_ref().expect("every drawn case solves");
        assert_eq!(case.refusal, None);
        assert_eq!(solution.guns.len(), case.inputs.guns.len());
        assert_eq!(
            (case.lead_recommended_rings, case.lead_time_of_flight_s),
            lead_summary(Some(solution))
        );
    }
}

#[test]
fn every_f64_of_the_inputs_and_solution_carries_its_bit_pattern() {
    let catalog = committed_catalog();
    let report = agreement_report(&catalog, SEED, 4);
    let case = report
        .cases
        .iter()
        .find(|case| case.lead_time_of_flight_s.is_some())
        .expect("a drawn case has a lead charge that solves");
    let solution = case.solution.as_ref().expect("the case solves");
    let lead = &solution.guns[0];
    assert_eq!(
        case.bit_patterns.get("/solution/guns/0/azimuth_mils"),
        Some(&format!("{:016x}", lead.azimuth_mils.to_bits()))
    );
    assert_eq!(
        case.bit_patterns.get("/inputs/target/x"),
        Some(&format!("{:016x}", case.inputs.target.x.to_bits()))
    );
    let solved_row = lead
        .charges
        .iter()
        .position(|charge| charge.time_of_flight_s.is_some())
        .expect("a charge solves");
    assert_eq!(
        case.bit_patterns.get(&format!(
            "/solution/guns/0/charges/{solved_row}/time_of_flight_s"
        )),
        Some(&format!(
            "{:016x}",
            lead.charges[solved_row].time_of_flight_s.unwrap().to_bits()
        ))
    );
    assert!(
        !case
            .bit_patterns
            .contains_key("/solution/guns/0/mils_per_circle")
    );
    assert!(!case.bit_patterns.contains_key("/inputs/catalog_version"));
    assert_eq!(
        case.bit_patterns,
        case_bit_patterns(&case.inputs, case.solution.as_ref())
    );
    assert_eq!(
        case.inputs,
        fire_mission_inputs(&catalog, &drawn_case(&catalog, case.case_id.as_str()))
    );
}

#[test]
fn the_reading_serialises_with_every_documented_key() {
    let catalog = committed_catalog();
    let value = serde_json::to_value(agreement_report(&catalog, SEED, 1)).expect("serialises");
    for key in [
        "seed",
        "count",
        "catalog_id",
        "catalog_version",
        "solver_revision",
        "cases",
    ] {
        assert!(value.get(key).is_some(), "the reading lacks `{key}`");
    }
    let case = &value["cases"][0];
    for key in [
        "case_id",
        "inputs",
        "solution",
        "refusal",
        "lead_recommended_rings",
        "lead_time_of_flight_s",
        "bit_patterns",
    ] {
        assert!(case.get(key).is_some(), "a case lacks `{key}`");
    }
}
