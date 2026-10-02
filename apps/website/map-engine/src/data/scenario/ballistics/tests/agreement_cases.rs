//! Tests of the agreement lattice: the SplitMix64 reference stream, determinism per seed, shell
//! coverage, the drawn ranges, that every drawn case is a valid battery request, and the shared
//! case-to-inputs mapping, lead summary and bit walk over the committed vanilla catalog.

use std::collections::BTreeSet;

use super::*;
use crate::data::scenario::ballistics::battery::solve_battery;
use crate::data::scenario::ballistics::fire_mission::solve_fire_mission;

/// The committed vanilla catalog the agreement gate solves natively; a missing file fails the
/// build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));
/// Seed of the mapping and bit-walk tests.
const MAPPING_SEED: u64 = 0x5EED_0000_0000_0001;

/// The hand-written test catalog: three shells, two weapons.
fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("../catalog/tests/minimal_catalog.json"))
        .expect("the sample catalog decodes")
}

#[test]
fn split_mix_64_matches_the_reference_stream() {
    // The published SplitMix64 outputs for seed 0.
    let mut draws = SplitMix64::new(0);
    assert_eq!(draws.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(draws.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(draws.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn unit_draws_stay_in_the_half_open_unit_interval() {
    let mut draws = SplitMix64::new(0x5EED);
    for _ in 0..10_000 {
        let unit = draws.next_unit();
        assert!((0.0..1.0).contains(&unit), "{unit}");
    }
    assert_eq!(
        SplitMix64::new(7).next_index(0),
        0,
        "a zero bound draws index 0"
    );
}

#[test]
fn the_lattice_is_deterministic_per_seed() {
    let catalog = catalog();
    let first = agreement_cases(&catalog, 0x00C0_FFEE, 24);
    let second = agreement_cases(&catalog, 0x00C0_FFEE, 24);
    assert_eq!(first, second);
    let bits = |cases: &[AgreementCase]| -> Vec<u64> {
        cases
            .iter()
            .flat_map(|case| {
                let mut values = vec![
                    case.target.x_m.to_bits(),
                    case.target.y_m.to_bits(),
                    case.target.height_m.to_bits(),
                    case.wind.speed_m_s.to_bits(),
                    case.wind.from_deg.to_bits(),
                ];
                for gun in &case.guns {
                    values.extend(gun.position_bits());
                }
                values
            })
            .collect()
    };
    assert_eq!(bits(&first), bits(&second), "bit for bit");
    assert_eq!(
        agreement_cases(&catalog, 0x00C0_FFEE, 10),
        first[..10],
        "a shorter lattice is a prefix of a longer one"
    );
    let other_seed = agreement_cases(&catalog, 0x00C0_FFEF, 24);
    assert_ne!(first, other_seed, "another seed draws other cases");
}

trait PositionBits {
    fn position_bits(&self) -> [u64; 3];
}

impl PositionBits for BatteryGun {
    fn position_bits(&self) -> [u64; 3] {
        [
            self.position.x_m.to_bits(),
            self.position.y_m.to_bits(),
            self.position.height_m.to_bits(),
        ]
    }
}

#[test]
fn the_lattice_covers_every_shell_a_weapon_fires() {
    let catalog = catalog();
    let every_shell: BTreeSet<&str> = catalog
        .shells
        .iter()
        .map(|shell| shell.shell_id.as_str())
        .collect();
    assert_eq!(every_shell.len(), 3);
    for seed in [0, 1, 0xDEAD_BEEF] {
        let cases = agreement_cases(&catalog, seed, every_shell.len());
        let drawn: BTreeSet<&str> = cases.iter().map(|case| case.shell_id.as_str()).collect();
        assert_eq!(drawn, every_shell, "seed {seed:#x}");
        for (index, case) in cases.iter().enumerate() {
            assert_eq!(
                case.shell_id, catalog.shells[index].shell_id,
                "catalog order"
            );
        }
    }
}

#[test]
fn a_shell_no_weapon_fires_is_skipped_and_an_unfired_catalog_draws_nothing() {
    let mut catalog = catalog();
    for weapon in &mut catalog.weapons {
        weapon.shell_ids.retain(|shell_id| shell_id != "o-832-he");
    }
    let cases = agreement_cases(&catalog, 3, 12);
    assert_eq!(cases.len(), 12);
    assert!(cases.iter().all(|case| case.shell_id != "o-832-he"));
    for weapon in &mut catalog.weapons {
        weapon.shell_ids.clear();
    }
    assert!(agreement_cases(&catalog, 3, 12).is_empty());
}

#[test]
fn every_case_is_well_formed() {
    let catalog = catalog();
    let seed = 0x0123_4567_89AB_CDEF;
    let cases = agreement_cases(&catalog, seed, 48);
    assert_eq!(cases.len(), 48);
    assert!(agreement_cases(&catalog, seed, 0).is_empty());
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case.case_id, format!("0123456789abcdef_{index:04}"));
        let weapon = catalog.weapon(&case.weapon_id).expect("a catalog weapon");
        assert!(weapon.fires_shell(&case.shell_id));
        assert!((1..=MAX_GUNS_PER_CASE as usize).contains(&case.guns.len()));
        for (gun_index, gun) in case.guns.iter().enumerate() {
            assert_eq!(gun.label, format!("gun-{}", gun_index + 1));
        }
        if index.is_multiple_of(4) {
            assert_eq!(case.wind, Wind::CALM);
        } else {
            assert!((0.0..MAX_WIND_SPEED_M_S).contains(&case.wind.speed_m_s));
            assert!((0.0..360.0).contains(&case.wind.from_deg));
        }
        assert!(case.wind.validate().is_ok());
        for value in [case.target.x_m, case.target.y_m, case.target.height_m] {
            assert!(value.is_finite());
        }
    }
    let gun_counts: BTreeSet<usize> = cases.iter().map(|case| case.guns.len()).collect();
    assert_eq!(
        gun_counts.len(),
        MAX_GUNS_PER_CASE as usize,
        "1, 2 and 3 guns all occur"
    );
}

#[test]
fn every_case_solves_as_a_battery_and_the_targets_span_the_charges() {
    let catalog = catalog();
    let cases = agreement_cases(&catalog, 0xB0B, 60);
    let mut recommended: BTreeSet<(String, Option<u32>)> = BTreeSet::new();
    let mut solved_guns = 0_usize;
    for case in &cases {
        let answers = solve_battery(&catalog, &case.battery_request())
            .unwrap_or_else(|error| panic!("case {} is a valid battery: {error}", case.case_id));
        assert_eq!(answers.len(), case.guns.len());
        for answer in &answers {
            recommended.insert((case.shell_id.clone(), answer.recommended_rings));
            solved_guns += usize::from(answer.recommended_rings.is_some());
        }
    }
    let total_guns: usize = cases.iter().map(|case| case.guns.len()).sum();
    assert!(
        solved_guns * 2 > total_guns,
        "most drawn guns solve ({solved_guns} of {total_guns})"
    );
    for shell in &catalog.shells {
        let rings: BTreeSet<Option<u32>> = recommended
            .iter()
            .filter(|(shell_id, _)| *shell_id == shell.shell_id)
            .map(|(_, rings)| *rings)
            .filter(Option::is_some)
            .collect();
        assert!(
            rings.len() >= 2,
            "ring-free targets for {} recommend more than one charge: {rings:?}",
            shell.shell_id
        );
    }
}

fn vanilla_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON).expect("the committed catalog decodes")
}

#[test]
fn the_inputs_restate_the_drawn_case_against_the_catalog() {
    let catalog = vanilla_catalog();
    let cases = agreement_cases(&catalog, MAPPING_SEED, 8);
    assert_eq!(cases.len(), 8);
    for case in &cases {
        let inputs = fire_mission_inputs(&catalog, case);
        assert_eq!(inputs.catalog_id, catalog.catalog_id);
        assert_eq!(inputs.catalog_version, catalog.catalog_version);
        assert_eq!(inputs.weapon_id, case.weapon_id);
        assert_eq!(inputs.shell_id, case.shell_id);
        assert_eq!(inputs.charge_rings, None);
        assert_eq!(inputs.burst_height_m, None);
        assert_eq!(inputs.crest_profile, None);
        assert_eq!(inputs.target.x.to_bits(), case.target.x_m.to_bits());
        assert_eq!(inputs.target.y.to_bits(), case.target.y_m.to_bits());
        assert_eq!(
            inputs.target.height_m.to_bits(),
            case.target.height_m.to_bits()
        );
        assert_eq!(inputs.target.height_source, HeightSource::Manual);
        assert_eq!(inputs.guns.len(), case.guns.len());
        for (gun, drawn) in inputs.guns.iter().zip(&case.guns) {
            assert_eq!(gun.label, drawn.label);
            assert_eq!(gun.x.to_bits(), drawn.position.x_m.to_bits());
            assert_eq!(gun.y.to_bits(), drawn.position.y_m.to_bits());
            assert_eq!(gun.height_m.to_bits(), drawn.position.height_m.to_bits());
            assert_eq!(gun.height_source, HeightSource::Manual);
        }
        let wind = inputs.wind.expect("the wind is always present");
        assert_eq!(wind.speed_m_s.to_bits(), case.wind.speed_m_s.to_bits());
        assert_eq!(wind.from_deg.to_bits(), case.wind.from_deg.to_bits());
    }
}

#[test]
fn the_lead_summary_is_the_recommended_row() {
    let catalog = vanilla_catalog();
    let mut timed = 0_usize;
    for case in agreement_cases(&catalog, MAPPING_SEED, 8) {
        let solution = solve_fire_mission(&catalog, &fire_mission_inputs(&catalog, &case))
            .unwrap_or_else(|refused| panic!("case {} solves: {refused}", case.case_id));
        let (rings, time_of_flight_s) = lead_summary(Some(&solution));
        let lead = &solution.guns[0];
        assert_eq!(rings, lead.recommended_rings);
        let row = lead
            .charges
            .iter()
            .find(|charge| Some(charge.rings) == rings);
        assert_eq!(
            time_of_flight_s.map(f64::to_bits),
            row.and_then(|charge| charge.time_of_flight_s)
                .map(f64::to_bits)
        );
        timed += usize::from(time_of_flight_s.is_some());
    }
    assert!(timed > 0, "a drawn case has a lead charge that solves");
    assert_eq!(lead_summary(None), (None, None));
}

#[test]
fn every_f64_of_the_inputs_and_solution_carries_its_bit_pattern() {
    let catalog = vanilla_catalog();
    let (inputs, solution) = agreement_cases(&catalog, MAPPING_SEED, 8)
        .iter()
        .map(|case| {
            let inputs = fire_mission_inputs(&catalog, case);
            let solution = solve_fire_mission(&catalog, &inputs).expect("the case solves");
            (inputs, solution)
        })
        .find(|(_, solution)| lead_summary(Some(solution)).1.is_some())
        .expect("a drawn case has a lead charge that solves");
    let patterns = case_bit_patterns(&inputs, Some(&solution));
    let hex = |value: f64| format!("{:016x}", value.to_bits());
    let lead = &solution.guns[0];
    assert_eq!(
        patterns.get("/solution/guns/0/azimuth_mils"),
        Some(&hex(lead.azimuth_mils))
    );
    assert_eq!(
        patterns.get("/inputs/target/x"),
        Some(&hex(inputs.target.x))
    );
    assert_eq!(
        patterns.get("/inputs/wind/from_deg"),
        Some(&hex(inputs.wind.expect("a wind").from_deg))
    );
    let solved_row = lead
        .charges
        .iter()
        .position(|charge| charge.time_of_flight_s.is_some())
        .expect("a charge solves");
    assert_eq!(
        patterns.get(&format!(
            "/solution/guns/0/charges/{solved_row}/time_of_flight_s"
        )),
        Some(&hex(lead.charges[solved_row]
            .time_of_flight_s
            .expect("the row solves")))
    );
    assert!(!patterns.contains_key("/solution/guns/0/mils_per_circle"));
    assert!(!patterns.contains_key("/inputs/catalog_version"));
    assert_eq!(
        patterns,
        f64_bit_patterns(&json!({ "inputs": inputs, "solution": solution }))
    );
    let refused = case_bit_patterns(&inputs, None);
    assert!(!refused.is_empty());
    assert!(
        refused
            .keys()
            .all(|pointer| pointer.starts_with("/inputs/"))
    );
    assert_eq!(
        refused,
        f64_bit_patterns(&json!({ "inputs": inputs, "solution": null }))
    );
}

#[test]
fn bit_patterns_escape_pointer_tokens_and_skip_integers_and_text() {
    let patterns = f64_bit_patterns(&json!({
        "a/b": 1.5,
        "c~d": [0.0, -0.0, 7, "x", null, true, 2.5],
    }));
    assert_eq!(patterns.len(), 4);
    assert_eq!(patterns["/a~1b"], format!("{:016x}", 1.5_f64.to_bits()));
    assert_eq!(patterns["/c~0d/0"], "0000000000000000");
    assert_eq!(patterns["/c~0d/1"], "8000000000000000");
    assert_eq!(patterns["/c~0d/6"], format!("{:016x}", 2.5_f64.to_bits()));
    assert!(f64_bit_patterns(&json!(3)).is_empty());
    assert_eq!(
        f64_bit_patterns(&json!(0.25)),
        BTreeMap::from([(String::new(), format!("{:016x}", 0.25_f64.to_bits()))])
    );
}
