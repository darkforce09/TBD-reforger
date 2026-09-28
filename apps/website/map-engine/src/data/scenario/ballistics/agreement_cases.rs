//! Agreement cases: a seeded, deterministic lattice of battery fire problems over a catalog, the
//! fire-mission inputs each case stands for, and the bit patterns of every `f64` it holds.
//!
//! **Role:** draws `count` battery problems (a catalog weapon and shell, a target placed without
//! regard to any charge, gun and target heights, a wind and one to three guns) from a SplitMix64
//! stream, so two builds of the solver (native and wasm32) can solve the same cases and compare
//! their answers bit for bit; turns a case into its [`FireMissionInputs`]
//! ([`fire_mission_inputs`]), restates a solution's lead gun ([`lead_summary`]) and records
//! every `f64` of the inputs and the solution by JSON pointer and bit pattern
//! ([`case_bit_patterns`]).
//!
//! **Position:** `data/scenario/ballistics`; reads a
//! [`crate::data::scenario::ballistics::catalog::BallisticsCatalog`] and hands each case to
//! [`crate::data::scenario::ballistics::battery::solve_battery`] through
//! [`AgreementCase::battery_request`] or to
//! [`crate::data::scenario::ballistics::fire_mission::solve_fire_mission`] through
//! [`fire_mission_inputs`]. The URL-only agreement bench of the single-page app and the native
//! agreement gate both draw with [`agreement_cases`] from the same seed and count and both map,
//! summarise and walk the cases with the functions here, so the two halves share one mapping.
//!
//! **Signals & state:** none; the generator state lives in a local [`SplitMix64`].
//!
//! **Invariants:**
//! - The same catalog, seed and count give the same cases on every target: draws are 64-bit
//!   integer arithmetic, a draw becomes a float as its top 53 bits times `2^-53` (exact), and
//!   the only transcendentals (`sin`, `cos` of the drawn bearings) go through `libm`.
//! - Case `i` fires shell `i mod n` of the `n` catalog shells some weapon fires, in catalog
//!   order, so any `count ≥ n` covers every such shell; the weapon is drawn among those that
//!   fire it.
//! - The target is "ring-free": its distance is drawn across the whole shell's reach, from the
//!   calm-air downrange of the slowest charge at the highest elevation to that of the fastest
//!   charge at the lowest elevation, so any charge (or none) may solve it.
//! - Every fourth case (`i mod 4 = 0`) is calm; the others draw a wind up to
//!   [`MAX_WIND_SPEED_M_S`] from any direction.
//! - Case identifiers are `<seed as 16 hex digits>_<index as 4 digits>`.
//! - A case's fire-mission inputs carry no operator charge, burst height or terrain profile,
//!   every height is `manual` and the wind is always present (calm cases carry a zero wind).
//! - A bit pattern is the 16 lowercase hexadecimal digits of an `f64`'s IEEE 754 bits, keyed by
//!   its RFC 6901 pointer under `{"inputs", "solution"}`; integers, text and nulls carry none.

use core::f64::consts::TAU;
use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::data::scenario::ballistics::battery::{BatteryGun, BatteryRequest};
use crate::data::scenario::ballistics::catalog::{
    BallisticsCatalog, Shell, WeaponSystem, flight_parameters, muzzle_speed_m_s,
};
use crate::data::scenario::ballistics::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionSolution,
    FireMissionWind, HeightSource,
};
use crate::data::scenario::ballistics::flight_model::{Launch, PathRecording, fly_to_height};
use crate::data::scenario::ballistics::solver::MapPosition;
use crate::data::scenario::ballistics::wind::Wind;

/// Strongest drawn wind, metres per second.
pub const MAX_WIND_SPEED_M_S: f64 = 12.0;
/// Most guns drawn for one case.
pub const MAX_GUNS_PER_CASE: u64 = 3;
/// Lowest drawn gun height, metres.
const GUN_HEIGHT_MIN_M: f64 = 0.0;
/// Span of the drawn gun height above [`GUN_HEIGHT_MIN_M`], metres.
const GUN_HEIGHT_SPAN_M: f64 = 300.0;
/// The target stands up to this many metres above or below the battery.
const TARGET_HEIGHT_OFFSET_M: f64 = 60.0;
/// Lowest drawn battery-centre coordinate, metres east and north.
const MAP_ORIGIN_M: f64 = 1_000.0;
/// Span of the drawn battery-centre coordinates, metres.
const MAP_SPAN_M: f64 = 10_000.0;
/// Shortest spacing between neighbouring guns, metres.
const GUN_SPACING_MIN_M: f64 = 20.0;
/// Span of the drawn gun spacing above [`GUN_SPACING_MIN_M`], metres.
const GUN_SPACING_SPAN_M: f64 = 40.0;
/// A gun stands up to this many metres above or below the battery height.
const GUN_HEIGHT_JITTER_M: f64 = 5.0;

/// The SplitMix64 generator: a 64-bit counter stepped by the golden-ratio increment and mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose first draw mixes `seed + 0x9E37_79B9_7F4A_7C15`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit draw.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// The next draw in `[0, 1)`: the top 53 bits scaled by `2^-53`, exact on every target.
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// The next draw in `[low, low + span)`.
    pub fn next_in(&mut self, low: f64, span: f64) -> f64 {
        low + span * self.next_unit()
    }

    /// The next draw in `0..bound`; `bound` is at least one.
    pub fn next_index(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound.max(1)
    }
}

/// One drawn battery fire problem.
#[derive(Debug, Clone, PartialEq)]
pub struct AgreementCase {
    /// `<seed as 16 hex digits>_<index as 4 digits>`.
    pub case_id: String,
    /// Launcher identifier in the catalog.
    pub weapon_id: String,
    /// Shell identifier in the catalog; the weapon fires it.
    pub shell_id: String,
    /// Where the shells must land.
    pub target: MapPosition,
    /// One to [`MAX_GUNS_PER_CASE`] guns, labelled `gun-1`, `gun-2`, ….
    pub guns: Vec<BatteryGun>,
    /// The surface wind; calm on every fourth case.
    pub wind: Wind,
}

impl AgreementCase {
    /// The case as a battery request borrowing its identifiers and guns.
    pub fn battery_request(&self) -> BatteryRequest<'_> {
        BatteryRequest {
            weapon_id: &self.weapon_id,
            shell_id: &self.shell_id,
            guns: &self.guns,
            target: self.target,
            wind: self.wind,
        }
    }
}

/// Draws `count` cases over `catalog` from `seed`; empty when no weapon fires any shell.
pub fn agreement_cases(catalog: &BallisticsCatalog, seed: u64, count: usize) -> Vec<AgreementCase> {
    let fired: Vec<(&Shell, Vec<&WeaponSystem>)> = catalog
        .shells
        .iter()
        .filter_map(|shell| {
            let weapons: Vec<&WeaponSystem> = catalog
                .weapons
                .iter()
                .filter(|weapon| weapon.fires_shell(&shell.shell_id))
                .collect();
            (!weapons.is_empty()).then_some((shell, weapons))
        })
        .collect();
    if fired.is_empty() {
        return Vec::new();
    }
    let mut draws = SplitMix64::new(seed);
    (0..count)
        .map(|index| {
            let (shell, weapons) = &fired[index % fired.len()];
            let weapon = weapons[draws.next_index(weapons.len() as u64) as usize];
            draw_case(&mut draws, catalog, weapon, shell, seed, index)
        })
        .collect()
}

fn draw_case(
    draws: &mut SplitMix64,
    catalog: &BallisticsCatalog,
    weapon: &WeaponSystem,
    shell: &Shell,
    seed: u64,
    index: usize,
) -> AgreementCase {
    let (near_m, far_m) = shell_reach_m(catalog, weapon, shell);
    let centre_east_m = draws.next_in(MAP_ORIGIN_M, MAP_SPAN_M);
    let centre_north_m = draws.next_in(MAP_ORIGIN_M, MAP_SPAN_M);
    let battery_height_m = draws.next_in(GUN_HEIGHT_MIN_M, GUN_HEIGHT_SPAN_M);
    let bearing_rad = draws.next_in(0.0, TAU);
    let distance_m = draws.next_in(near_m, far_m - near_m);
    let target_height_m = draws.next_in(
        battery_height_m - TARGET_HEIGHT_OFFSET_M,
        2.0 * TARGET_HEIGHT_OFFSET_M,
    );
    let (sin_bearing, cos_bearing) = (libm::sin(bearing_rad), libm::cos(bearing_rad));
    let target = MapPosition {
        x_m: centre_east_m + distance_m * sin_bearing,
        y_m: centre_north_m + distance_m * cos_bearing,
        height_m: target_height_m,
    };

    let gun_count = 1 + draws.next_index(MAX_GUNS_PER_CASE);
    let spacing_m = draws.next_in(GUN_SPACING_MIN_M, GUN_SPACING_SPAN_M);
    let guns = (0..gun_count)
        .map(|gun_index| {
            // Guns stand on a line across the line of fire, centred on the battery centre.
            let across_m = (gun_index as f64 - (gun_count - 1) as f64 / 2.0) * spacing_m;
            let height_m =
                battery_height_m + draws.next_in(-GUN_HEIGHT_JITTER_M, 2.0 * GUN_HEIGHT_JITTER_M);
            BatteryGun {
                label: format!("gun-{}", gun_index + 1),
                position: MapPosition {
                    x_m: centre_east_m + across_m * cos_bearing,
                    y_m: centre_north_m - across_m * sin_bearing,
                    height_m,
                },
            }
        })
        .collect();

    let wind = if index.is_multiple_of(4) {
        Wind::CALM
    } else {
        Wind {
            speed_m_s: draws.next_in(0.0, MAX_WIND_SPEED_M_S),
            from_deg: draws.next_in(0.0, 360.0),
        }
    };
    AgreementCase {
        case_id: format!("{seed:016x}_{index:04}"),
        weapon_id: weapon.weapon_id.clone(),
        shell_id: shell.shell_id.clone(),
        target,
        guns,
        wind,
    }
}

/// The shell's reach on level ground in calm air: the downrange of its slowest charge at the
/// weapon's highest elevation and of its fastest charge at the lowest elevation, metres, with
/// `near ≤ far`. A flight that does not land falls back to the vacuum range of that launch.
fn shell_reach_m(catalog: &BallisticsCatalog, weapon: &WeaponSystem, shell: &Shell) -> (f64, f64) {
    let (lowest_rad, highest_rad) = weapon.elevation_limits_rad();
    let speeds = shell
        .charges
        .iter()
        .map(|charge| muzzle_speed_m_s(weapon, shell, charge));
    let slowest = speeds.clone().fold(f64::INFINITY, f64::min);
    let fastest = speeds.fold(0.0, f64::max);
    let parameters = flight_parameters(catalog.gravity_m_s2, shell);
    let reach = |muzzle_speed_m_s: f64, elevation_rad: f64| -> f64 {
        let launch = Launch {
            muzzle_speed_m_s,
            elevation_rad,
            azimuth_rad: 0.0,
        };
        match fly_to_height(
            &parameters,
            &launch,
            &Wind::CALM,
            0.0,
            PathRecording::Discard,
        ) {
            Ok(flight) => flight.downrange_m,
            Err(_) => {
                muzzle_speed_m_s * muzzle_speed_m_s * libm::sin(2.0 * elevation_rad)
                    / catalog.gravity_m_s2
            }
        }
    };
    let near_m = reach(slowest, highest_rad);
    let far_m = reach(fastest, lowest_rad);
    let (near_m, far_m) = if near_m.is_finite() && far_m.is_finite() {
        (near_m.min(far_m).max(0.0), near_m.max(far_m).max(0.0))
    } else {
        (0.0, 0.0)
    };
    (near_m, far_m)
}

/// The fire-mission inputs a drawn case stands for against `catalog`: its weapon, shell, target,
/// guns and wind, every height `manual`, no operator charge, burst height or terrain profile.
pub fn fire_mission_inputs(catalog: &BallisticsCatalog, case: &AgreementCase) -> FireMissionInputs {
    FireMissionInputs {
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        weapon_id: case.weapon_id.clone(),
        shell_id: case.shell_id.clone(),
        charge_rings: None,
        target: FireMissionPoint {
            x: case.target.x_m,
            y: case.target.y_m,
            height_m: case.target.height_m,
            height_source: HeightSource::Manual,
        },
        guns: case
            .guns
            .iter()
            .map(|gun| FireMissionGunPosition {
                label: gun.label.clone(),
                x: gun.position.x_m,
                y: gun.position.y_m,
                height_m: gun.position.height_m,
                height_source: HeightSource::Manual,
            })
            .collect(),
        wind: Some(FireMissionWind {
            speed_m_s: case.wind.speed_m_s,
            from_deg: case.wind.from_deg,
        }),
        burst_height_m: None,
        crest_profile: None,
    }
}

/// The lead gun's recommended rings and that charge's time of flight; both `None` when there is
/// no solution or no gun, the time `None` when no charge solves.
pub fn lead_summary(solution: Option<&FireMissionSolution>) -> (Option<u32>, Option<f64>) {
    let Some(lead) = solution.and_then(|solution| solution.guns.first()) else {
        return (None, None);
    };
    let time_of_flight_s = lead.recommended_rings.and_then(|rings| {
        lead.charges
            .iter()
            .find(|charge| charge.rings == rings)
            .and_then(|charge| charge.time_of_flight_s)
    });
    (lead.recommended_rings, time_of_flight_s)
}

/// The bit patterns of a case: every `f64` of `{"inputs": inputs, "solution": solution}`, a
/// refused case (`solution` `None`) carrying patterns for its inputs only.
pub fn case_bit_patterns(
    inputs: &FireMissionInputs,
    solution: Option<&FireMissionSolution>,
) -> BTreeMap<String, String> {
    f64_bit_patterns(&json!({ "inputs": inputs, "solution": solution }))
}

/// The RFC 6901 pointer of every `f64` leaf of `value` mapped to its IEEE 754 bits as 16
/// lowercase hexadecimal digits.
pub fn f64_bit_patterns(value: &Value) -> BTreeMap<String, String> {
    let mut patterns = BTreeMap::new();
    collect_f64_bits(value, &mut String::new(), &mut patterns);
    patterns
}

/// Walks `value` depth first; `pointer` is the pointer of `value` on entry and on return.
fn collect_f64_bits(value: &Value, pointer: &mut String, patterns: &mut BTreeMap<String, String>) {
    let depth = pointer.len();
    match value {
        Value::Number(number) if number.is_f64() => {
            if let Some(float) = number.as_f64() {
                patterns.insert(pointer.clone(), format!("{:016x}", float.to_bits()));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                pointer.push('/');
                pointer.push_str(&index.to_string());
                collect_f64_bits(item, pointer, patterns);
                pointer.truncate(depth);
            }
        }
        Value::Object(fields) => {
            for (key, item) in fields {
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                collect_f64_bits(item, pointer, patterns);
                pointer.truncate(depth);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "tests/agreement_cases.rs"]
mod tests;
