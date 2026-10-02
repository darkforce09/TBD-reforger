//! Bounded failure of the firing solver: 10,000 seeded random requests against the vanilla
//! catalog, clean or with one corrupted value (NaN, infinities, zero, negative, huge or tiny
//! positions, heights, wind, gravity, shell constants, charge and weapon values, identifiers),
//! each solved without a panic into either a typed [`FireSolutionError`] or a solution whose
//! every charge row is coherent: all values finite and within the weapon's limits with no
//! refusal, or no value and a typed refusal. Ten tests of 1,000 draws run in parallel.

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::data::scenario::ballistics::agreement_cases::SplitMix64;
use crate::data::scenario::ballistics::catalog::BallisticsCatalog;
use crate::data::scenario::ballistics::solver::{
    FireSolution, FireSolutionError, FireSolutionRequest, MapPosition, SolutionRefusal,
    solve_fire_solution,
};
use crate::data::scenario::ballistics::wind::Wind;

/// The committed vanilla catalog; a missing file fails the build.
const VANILLA_CATALOG_JSON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));

/// Draws per test; ten tests make the 10,000.
const DRAWS_PER_TEST: usize = 1_000;

/// Seed of the first test; test `n` draws from `FIRST_SEED + n`.
const FIRST_SEED: u64 = 0xB0B0_0000_0000_2026;

/// The values a corrupted field takes: every class of input a request must survive.
const CORRUPT_VALUES: [f64; 9] = [
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    0.0,
    -1.0,
    1e300,
    -1e300,
    f64::MIN_POSITIVE,
    1e-300,
];

/// How many fields [`corrupt`] can spoil.
const CORRUPTIBLE_FIELDS: u64 = 22;

/// One drawn request with the catalog it is solved against.
struct Draw {
    catalog: BallisticsCatalog,
    weapon_id: String,
    shell_id: String,
    gun: MapPosition,
    target: MapPosition,
    wind: Wind,
    corrupted: bool,
}

/// What one test's draws produced.
#[derive(Default)]
struct Tally {
    solved_rows: usize,
    refusals: BTreeSet<&'static str>,
    errors: BTreeSet<&'static str>,
    failures: Vec<String>,
}

fn corrupt_value(draws: &mut SplitMix64) -> f64 {
    CORRUPT_VALUES[draws.next_index(CORRUPT_VALUES.len() as u64) as usize]
}

/// A clean request: a catalog weapon and one of its shells, a gun, a target up to 3 km away
/// and 200 m above or below, and calm air or a wind up to 15 m/s.
fn clean_draw(vanilla: &BallisticsCatalog, draws: &mut SplitMix64) -> Draw {
    let weapon = &vanilla.weapons[draws.next_index(vanilla.weapons.len() as u64) as usize];
    let shell_id =
        weapon.shell_ids[draws.next_index(weapon.shell_ids.len() as u64) as usize].clone();
    let gun = MapPosition {
        x_m: draws.next_in(-5_000.0, 10_000.0),
        y_m: draws.next_in(-5_000.0, 10_000.0),
        height_m: draws.next_in(0.0, 400.0),
    };
    let distance_m = if draws.next_index(50) == 0 {
        0.0
    } else {
        draws.next_in(0.0, 3_000.0)
    };
    let bearing = draws.next_in(0.0, core::f64::consts::TAU);
    let target = MapPosition {
        x_m: gun.x_m + distance_m * bearing.sin(),
        y_m: gun.y_m + distance_m * bearing.cos(),
        height_m: gun.height_m + draws.next_in(-200.0, 400.0),
    };
    let wind = if draws.next_index(4) == 0 {
        Wind::CALM
    } else {
        Wind {
            speed_m_s: draws.next_in(0.0, 15.0),
            from_deg: draws.next_in(0.0, 360.0),
        }
    };
    Draw {
        catalog: vanilla.clone(),
        weapon_id: weapon.weapon_id.clone(),
        shell_id,
        gun,
        target,
        wind,
        corrupted: false,
    }
}

/// Spoils one field of `draw`: a request value, the catalog's gravity, a constant of the drawn
/// shell, weapon or charge, or an identifier.
fn corrupt(draw: &mut Draw, draws: &mut SplitMix64) {
    draw.corrupted = true;
    let field = draws.next_index(CORRUPTIBLE_FIELDS);
    let value = corrupt_value(draws);
    let catalog = &mut draw.catalog;
    let shell_index = catalog
        .shells
        .iter()
        .position(|shell| shell.shell_id == draw.shell_id)
        .expect("a drawn shell is in the catalog");
    let weapon_index = catalog
        .weapons
        .iter()
        .position(|weapon| weapon.weapon_id == draw.weapon_id)
        .expect("a drawn weapon is in the catalog");
    let charge_count = catalog.shells[shell_index].charges.len() as u64;
    let charge_index = draws.next_index(charge_count) as usize;
    let shell = &mut catalog.shells[shell_index];
    let weapon = &mut catalog.weapons[weapon_index];
    match field {
        0 => draw.gun.x_m = value,
        1 => draw.gun.y_m = value,
        2 => draw.gun.height_m = value,
        3 => draw.target.x_m = value,
        4 => draw.target.y_m = value,
        5 => draw.target.height_m = value,
        6 => draw.wind.speed_m_s = value,
        7 => draw.wind.from_deg = value,
        8 => catalog.gravity_m_s2 = value,
        9 => shell.mass_kg = value,
        10 => shell.air_drag = value,
        11 => shell.init_speed_m_s = value,
        12 => shell.wind_influence_multiplier = value,
        13 => shell.time_to_live_s = value,
        14 => shell.charges[charge_index].init_speed_coef = value,
        15 => weapon.muzzle_init_speed_coef = value,
        16 => weapon.elevation_min_deg = value,
        17 => weapon.elevation_max_deg = value,
        18 => weapon.mils_per_circle = 0,
        19 => draw.weapon_id = "unknown-launcher".to_owned(),
        20 => draw.shell_id = "unknown-shell".to_owned(),
        _ => shell.charges.clear(),
    }
}

fn refusal_name(refusal: SolutionRefusal) -> &'static str {
    match refusal {
        SolutionRefusal::TooClose => "too_close",
        SolutionRefusal::OutOfRange => "out_of_range",
        SolutionRefusal::Unreachable => "unreachable",
        SolutionRefusal::DidNotConverge => "did_not_converge",
        SolutionRefusal::TimeToLiveExceeded => "time_to_live_exceeded",
        SolutionRefusal::InvalidInput => "invalid_input",
    }
}

fn error_name(error: &FireSolutionError) -> &'static str {
    match error {
        FireSolutionError::Lookup(_) => "lookup",
        FireSolutionError::InvalidInput { .. } => "invalid_input",
        FireSolutionError::InvalidWind(_) => "invalid_wind",
        FireSolutionError::InvalidMilsConvention(_) => "invalid_mils_convention",
        FireSolutionError::InvalidFlightParameters(_) => "invalid_flight_parameters",
    }
}

/// Every incoherence of `solution` for `draw`, as text.
fn incoherences(draw: &Draw, solution: &FireSolution, tally: &mut Tally) -> Vec<String> {
    let mut found = Vec::new();
    let weapon = draw
        .catalog
        .weapon(&draw.weapon_id)
        .expect("a solved weapon exists");
    let shell = draw
        .catalog
        .shell(&draw.shell_id)
        .expect("a solved shell exists");
    let mils_per_circle = f64::from(solution.mils_per_circle);
    let in_turn = |value: f64, turn: f64| value.is_finite() && (0.0..turn).contains(&value);
    let geometry_is_finite = solution.distance_m.is_finite()
        && solution.distance_m >= 0.0
        && solution.height_difference_m.is_finite();
    if !geometry_is_finite
        || !in_turn(solution.azimuth_deg, 360.0)
        || !in_turn(solution.azimuth_mils, mils_per_circle)
    {
        found.push(format!("non-finite or unfolded geometry {solution:?}"));
    }
    let catalog_rings: Vec<u32> = shell.charges.iter().map(|charge| charge.rings).collect();
    let row_rings: Vec<u32> = solution.charges.iter().map(|row| row.rings).collect();
    if catalog_rings != row_rings {
        found.push(format!(
            "rows {row_rings:?} are not the charges {catalog_rings:?}"
        ));
    }
    for row in &solution.charges {
        let values = [
            row.elevation_deg,
            row.elevation_mils,
            row.time_of_flight_s,
            row.apex_m,
            row.aim_azimuth_deg,
            row.aim_azimuth_mils,
            row.deflection_correction_mils,
            row.range_correction_m,
        ];
        match row.refusal {
            Some(refusal) => {
                tally.refusals.insert(refusal_name(refusal));
                if values.iter().any(Option::is_some) {
                    found.push(format!("refused row carries values {row:?}"));
                }
            }
            None => {
                tally.solved_rows += 1;
                let (Some(elevation_deg), Some(time_of_flight_s), Some(aim_deg), Some(aim_mils)) = (
                    row.elevation_deg,
                    row.time_of_flight_s,
                    row.aim_azimuth_deg,
                    row.aim_azimuth_mils,
                ) else {
                    found.push(format!("solved row misses a value {row:?}"));
                    continue;
                };
                if values
                    .iter()
                    .any(|value| !value.is_some_and(f64::is_finite))
                    || elevation_deg < weapon.elevation_min_deg - 1e-9
                    || elevation_deg > weapon.elevation_max_deg + 1e-9
                    || time_of_flight_s <= 0.0
                    || !in_turn(aim_deg, 360.0)
                    || !in_turn(aim_mils, mils_per_circle)
                {
                    found.push(format!("solved row out of its domain {row:?}"));
                }
            }
        }
    }
    let fewest_solving = solution
        .charges
        .iter()
        .filter(|row| row.solves())
        .map(|row| row.rings)
        .min();
    if solution.recommended_rings != fewest_solving {
        found.push(format!(
            "recommended {:?}, the fewest rings that solve are {fewest_solving:?}",
            solution.recommended_rings
        ));
    }
    found
}

/// Solves `DRAWS_PER_TEST` draws from `seed`; a third of them corrupted.
fn sweep(seed: u64) -> Tally {
    let vanilla = BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON)
        .expect("the vanilla catalog decodes");
    let mut draws = SplitMix64::new(seed);
    let mut tally = Tally::default();
    for index in 0..DRAWS_PER_TEST {
        let mut draw = clean_draw(&vanilla, &mut draws);
        if draws.next_index(3) == 0 {
            corrupt(&mut draw, &mut draws);
        }
        let request = FireSolutionRequest {
            weapon_id: &draw.weapon_id,
            shell_id: &draw.shell_id,
            gun: draw.gun,
            target: draw.target,
            wind: draw.wind,
        };
        let case = format!(
            "seed {seed:#x} draw {index}: {} / {} gun {:?} target {:?} wind {:?}",
            draw.weapon_id, draw.shell_id, draw.gun, draw.target, draw.wind
        );
        match catch_unwind(AssertUnwindSafe(|| {
            solve_fire_solution(&draw.catalog, &request)
        })) {
            Err(_) => tally.failures.push(format!("{case}: panicked")),
            Ok(Err(error)) => {
                tally.errors.insert(error_name(&error));
                if !draw.corrupted {
                    tally
                        .failures
                        .push(format!("{case}: a clean request is refused: {error}"));
                }
            }
            Ok(Ok(solution)) => {
                for incoherence in incoherences(&draw, &solution, &mut tally) {
                    tally.failures.push(format!("{case}: {incoherence}"));
                }
            }
        }
    }
    tally
}

/// Every draw of test `offset` survives and is typed; the draws reach solved rows, several
/// refusal kinds and request-wide errors.
fn assert_sweep_is_bounded(offset: u64) {
    let tally = sweep(FIRST_SEED + offset);
    assert!(
        tally.failures.is_empty(),
        "{} failures, first: {:#?}",
        tally.failures.len(),
        &tally.failures[..tally.failures.len().min(10)]
    );
    assert!(tally.solved_rows > 0, "no draw solved a charge");
    assert!(
        tally.refusals.len() >= 3,
        "only the refusals {:?} were reached",
        tally.refusals
    );
    assert!(
        tally.errors.len() >= 3,
        "only the request errors {:?} were reached",
        tally.errors
    );
}

#[test]
fn random_requests_seed_0_are_bounded_and_typed() {
    assert_sweep_is_bounded(0);
}

#[test]
fn random_requests_seed_1_are_bounded_and_typed() {
    assert_sweep_is_bounded(1);
}

#[test]
fn random_requests_seed_2_are_bounded_and_typed() {
    assert_sweep_is_bounded(2);
}

#[test]
fn random_requests_seed_3_are_bounded_and_typed() {
    assert_sweep_is_bounded(3);
}

#[test]
fn random_requests_seed_4_are_bounded_and_typed() {
    assert_sweep_is_bounded(4);
}

#[test]
fn random_requests_seed_5_are_bounded_and_typed() {
    assert_sweep_is_bounded(5);
}

#[test]
fn random_requests_seed_6_are_bounded_and_typed() {
    assert_sweep_is_bounded(6);
}

#[test]
fn random_requests_seed_7_are_bounded_and_typed() {
    assert_sweep_is_bounded(7);
}

#[test]
fn random_requests_seed_8_are_bounded_and_typed() {
    assert_sweep_is_bounded(8);
}

#[test]
fn random_requests_seed_9_are_bounded_and_typed() {
    assert_sweep_is_bounded(9);
}

#[test]
fn a_target_on_the_gun_itself_is_refused_by_every_charge() {
    let catalog = BallisticsCatalog::from_json_slice(VANILLA_CATALOG_JSON)
        .expect("the vanilla catalog decodes");
    let gun = MapPosition {
        x_m: -3_220.5,
        y_m: -2_129.25,
        height_m: 69.75,
    };
    let mut solved = Vec::new();
    for weapon in &catalog.weapons {
        for shell_id in &weapon.shell_ids {
            for wind in [
                Wind::CALM,
                Wind {
                    speed_m_s: 8.875,
                    from_deg: 105.5,
                },
            ] {
                let request = FireSolutionRequest {
                    weapon_id: &weapon.weapon_id,
                    shell_id,
                    gun,
                    target: gun,
                    wind,
                };
                let solution =
                    solve_fire_solution(&catalog, &request).expect("a valid request solves");
                solved.extend(
                    solution
                        .charges
                        .iter()
                        .filter(|row| row.solves())
                        .map(|row| format!("{}/{shell_id} {wind:?}: {row:?}", weapon.weapon_id)),
                );
            }
        }
    }
    assert!(solved.is_empty(), "{solved:#?}");
}
