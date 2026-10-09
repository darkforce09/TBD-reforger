//! Tests of the agreement lattice over the committed vanilla catalog: determinism per seed and
//! that every drawn case is well formed.

use std::collections::BTreeSet;

use super::*;

/// The hand-written test catalog: three shells, two weapons.
fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/ballistics/minimal_catalog.json"
    )))
    .expect("the sample catalog decodes")
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
fn every_case_is_well_formed() {
    let catalog = catalog();
    let seed = 0x0123_4567_89AB_CDEF;
    let cases = agreement_cases(&catalog, seed, 48);
    assert_eq!(cases.len(), 48);
    assert!(agreement_cases(&catalog, seed, 0).is_empty());
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case.case_id, *format!("0123456789abcdef_{index:04}"));
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
