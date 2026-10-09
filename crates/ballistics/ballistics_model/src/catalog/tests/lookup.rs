//! Tests of the typed catalog lookups and the flight parameters of one firing.

use super::*;
use crate::angular_units::MilsConvention;
use crate::ids::{ShellId, WeaponId};

fn sample_catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/ballistics/minimal_catalog.json"
    )))
    .expect("the sample catalog decodes")
}

#[test]
fn known_weapon_and_shell_are_found() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog
            .weapon(&WeaponId::new("m252"))
            .map(|w| &*w.display_name),
        Ok("M252 81mm")
    );
    assert_eq!(
        catalog.shell(&ShellId::new("o-832-he")).map(|s| s.mass_kg),
        Ok(3.1)
    );
}

#[test]
fn unknown_weapon_is_refused_before_anything_else() {
    let catalog = sample_catalog();
    let expected = Err(CatalogLookupError::UnknownWeapon {
        weapon_id: WeaponId::new("M252"),
    });
    assert_eq!(
        catalog.weapon(&WeaponId::new("M252")).map(|_| ()),
        expected.clone()
    );
    assert_eq!(
        catalog
            .resolve_firing(&WeaponId::new("M252"), &ShellId::new("no-such-shell"), 99)
            .map(|_| ()),
        expected
    );
}

#[test]
fn shell_the_weapon_does_not_fire_is_incompatible() {
    let catalog = sample_catalog();
    assert_eq!(
        catalog
            .weapon_and_shell(&WeaponId::new("m252"), &ShellId::new("o-832-he"))
            .map(|_| ()),
        Err(CatalogLookupError::IncompatibleShell {
            weapon_id: WeaponId::new("m252"),
            shell_id: ShellId::new("o-832-he")
        })
    );
    assert_eq!(
        catalog
            .resolve_firing(&WeaponId::new("2b14"), &ShellId::new("m821-he"), 0)
            .map(|_| ()),
        Err(CatalogLookupError::IncompatibleShell {
            weapon_id: WeaponId::new("2b14"),
            shell_id: ShellId::new("m821-he")
        })
    );
}

#[test]
fn resolved_firing_carries_the_shell_constants_and_muzzle_speed() {
    let catalog = sample_catalog();
    let firing = catalog
        .resolve_firing(&WeaponId::new("2b14"), &ShellId::new("o-832-he"), 4)
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
