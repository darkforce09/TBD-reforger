//! Tests of the typed catalog lookups and the flight parameters of one firing.

use super::*;
use crate::data::scenario::ballistics::angular_units::MilsConvention;

fn sample_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("minimal_catalog.json"))
        .expect("the sample catalog decodes")
}

#[test]
fn known_weapon_and_shell_are_found() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog.weapon("m252").map(|w| &*w.display_name),
        Ok("M252 81mm")
    );
    assert_eq!(catalog.shell("o-832-he").map(|s| s.mass_kg), Ok(3.1));
}

#[test]
fn unknown_weapon_is_refused_before_anything_else() {
    let catalog = sample_catalog();
    let expected = Err(CatalogLookupError::UnknownWeapon {
        weapon_id: "M252".to_owned(),
    });
    assert_eq!(catalog.weapon("M252").map(|_| ()), expected.clone());
    assert_eq!(
        catalog
            .resolve_firing("M252", "no-such-shell", 99)
            .map(|_| ()),
        expected
    );
}

#[test]
fn unknown_shell_is_refused_before_compatibility() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog.resolve_firing("m252", "m821", 0).map(|_| ()),
        Err(CatalogLookupError::UnknownShell {
            shell_id: "m821".to_owned()
        })
    );
}

#[test]
fn shell_the_weapon_does_not_fire_is_incompatible() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog.weapon_and_shell("m252", "o-832-he").map(|_| ()),
        Err(CatalogLookupError::IncompatibleShell {
            weapon_id: "m252".to_owned(),
            shell_id: "o-832-he".to_owned()
        })
    );
    assert_eq!(
        catalog.resolve_firing("2b14", "m821-he", 0).map(|_| ()),
        Err(CatalogLookupError::IncompatibleShell {
            weapon_id: "2b14".to_owned(),
            shell_id: "m821-he".to_owned()
        })
    );
}

#[test]
fn ring_count_the_shell_does_not_accept_is_unknown() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog.resolve_firing("2b14", "o-832-he", 2).map(|_| ()),
        Err(CatalogLookupError::UnknownRing {
            shell_id: "o-832-he".to_owned(),
            rings: 2
        })
    );
}

#[test]
fn resolved_firing_carries_the_shell_constants_and_muzzle_speed() {
    let catalog = sample_catalog();
    let firing = catalog
        .resolve_firing("2b14", "o-832-he", 4)
        .expect("the 2B14 fires the O-832 at four rings");
    assert_eq!(firing.weapon.weapon_id, "2b14");
    assert_eq!(firing.shell.shell_id, "o-832-he");
    assert_eq!(firing.charge.rings, 4);
    assert_eq!(firing.muzzle_speed_m_s, 72.0 * 2.5 * 0.95);
    assert_eq!(
        firing.flight_parameters,
        FlightParameters {
            gravity_m_s2: 9.81,
            mass_kg: 3.1,
            air_drag: 0.004,
            wind_influence_multiplier: 0.0,
            time_to_live_s: 60.0,
            integration_step_s: DEFAULT_INTEGRATION_STEP_S,
        }
    );
    assert_eq!(firing.flight_parameters.validate(), Ok(()));
    assert_eq!(firing.weapon.mils_convention(), MilsConvention::new(6000));
}

#[test]
fn every_listed_shell_and_charge_of_the_sample_resolves() {
    let catalog = sample_catalog();
    for weapon in &catalog.weapons {
        for shell_id in &weapon.shell_ids {
            let shell = catalog.shell(shell_id).expect("listed shells exist");
            for charge in &shell.charges {
                let firing = catalog
                    .resolve_firing(&weapon.weapon_id, shell_id, charge.rings)
                    .expect("every listed combination resolves");
                assert_eq!(
                    firing.muzzle_speed_m_s,
                    shell.init_speed_m_s * charge.init_speed_coef * weapon.muzzle_init_speed_coef
                );
            }
        }
    }
}

#[test]
fn side_air_drag_scale_never_enters_the_flight_parameters() {
    let mut catalog = sample_catalog();
    let baseline = catalog
        .resolve_firing("m252", "m821-he", 1)
        .expect("resolves")
        .flight_parameters;
    for side_air_drag_scale in [0.0, 1.25, 7.5, 1_000.0] {
        catalog.shells[0].side_air_drag_scale = side_air_drag_scale;
        let parameters = catalog
            .resolve_firing("m252", "m821-he", 1)
            .expect("resolves")
            .flight_parameters;
        assert_eq!(parameters, baseline, "side scale {side_air_drag_scale}");
    }
    // Exhaustive destructuring: a side-drag field added to `FlightParameters` fails this build.
    let FlightParameters {
        gravity_m_s2,
        mass_kg,
        air_drag,
        wind_influence_multiplier,
        time_to_live_s,
        integration_step_s,
    } = baseline;
    let shell = &catalog.shells[0];
    assert_eq!(
        [
            gravity_m_s2,
            mass_kg,
            air_drag,
            wind_influence_multiplier,
            time_to_live_s
        ],
        [
            9.81,
            shell.mass_kg,
            shell.air_drag,
            shell.wind_influence_multiplier,
            shell.time_to_live_s
        ]
    );
    assert_eq!(integration_step_s, DEFAULT_INTEGRATION_STEP_S);
}
