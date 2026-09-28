//! The mismatch rule between a client's fire-mission solution and the server's re-solve.
//!
//! **Role:** compares two [`FireMissionSolution`]s and lists every difference the rule does
//! not tolerate: provenance (catalog id, catalog version, solver revision), the gun count, each
//! gun's weapon convention, charge rows and recommended charge, each charge row's refusal, aim
//! azimuth, elevation and time of flight, and the lead gun's fuze (refusal, fuze time and
//! burst-point aim, compared like a charge row).
//!
//! **Position:** `data/scenario/ballistics`; the API calls [`compare_solutions`] on a saved
//! fire mission's `client_solution` and its own
//! [`crate::data::scenario::ballistics::fire_mission::solve_fire_mission`] answer, and answers
//! `solution_mismatch` with the listed [`SolutionMismatch`]es when any exists.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Angles are compared in the weapon's mils: aim azimuths by their circular difference over
//!   `mils_per_circle`, elevations directly, each within [`ANGLE_TOLERANCE_MILS`]; times of
//!   flight and fuze times within [`TIME_TOLERANCE_S`].
//! - A value present on one side and absent on the other, or NaN on either side, is a
//!   mismatch; the gun count, ring lists, recommended rings, refusals and provenance must be
//!   equal.
//! - Guns are compared by position, charge rows by position within a gun once the ring lists
//!   agree; the comparison is symmetric in which values it flags.

use serde::{Deserialize, Serialize};

use crate::data::scenario::ballistics::fire_mission::{FireMissionFuze, FireMissionSolution};
use crate::data::scenario::ballistics::fuze::FuzeRefusal;
use crate::data::scenario::ballistics::solver::{ChargeSolution, SolutionRefusal};

/// Largest tolerated aim-azimuth or elevation difference, in the weapon's mils.
pub const ANGLE_TOLERANCE_MILS: f64 = 1.0;

/// Largest tolerated time-of-flight or fuze-time difference, seconds.
pub const TIME_TOLERANCE_S: f64 = 0.1;

/// Every intolerable difference between a client and a server solution.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolutionComparison {
    /// The differences, in comparison order; empty when the solutions agree.
    pub mismatches: Vec<SolutionMismatch>,
}

impl SolutionComparison {
    /// Whether the client solution is accepted.
    pub fn agrees(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// A provenance field of a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceField {
    /// The catalog id.
    CatalogId,
    /// The catalog version.
    CatalogVersion,
    /// The solver revision.
    SolverRevision,
}

/// The row a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "row", rename_all = "snake_case")]
pub enum ComparedRow {
    /// Charge `rings` of gun `gun_index`.
    GunCharge {
        /// Position of the gun in the solution.
        gun_index: usize,
        /// Rings of the charge.
        rings: u32,
    },
    /// The lead gun's fuze and its burst-point aim.
    Fuze,
}

/// A compared numeric quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparedQuantity {
    /// Aim azimuth, weapon mils.
    AimAzimuthMils,
    /// Elevation, weapon mils.
    ElevationMils,
    /// Time of flight, seconds.
    TimeOfFlightS,
    /// Fuze time, seconds.
    FuzeTimeS,
}

/// One intolerable difference, with the client's and the server's values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SolutionMismatch {
    /// A provenance field differs.
    Provenance {
        /// Which field.
        field: ProvenanceField,
        /// The client's value.
        client: String,
        /// The server's value.
        server: String,
    },
    /// The solutions carry different numbers of guns.
    GunCount {
        /// The client's count.
        client: usize,
        /// The server's count.
        server: usize,
    },
    /// A gun's weapon convention differs from the other side's or from the weapon's.
    MilsPerCircle {
        /// Position of the gun.
        gun_index: usize,
        /// The client's value.
        client: u32,
        /// The server's value.
        server: u32,
    },
    /// A gun's charge rows name different rings.
    ChargeRings {
        /// Position of the gun.
        gun_index: usize,
        /// The client's rings, in row order.
        client: Vec<u32>,
        /// The server's rings, in row order.
        server: Vec<u32>,
    },
    /// A gun's recommended charge differs.
    RecommendedCharge {
        /// Position of the gun.
        gun_index: usize,
        /// The client's rings.
        client: Option<u32>,
        /// The server's rings.
        server: Option<u32>,
    },
    /// A charge row's refusal differs.
    ChargeRefusal {
        /// The row.
        row: ComparedRow,
        /// The client's refusal.
        client: Option<SolutionRefusal>,
        /// The server's refusal.
        server: Option<SolutionRefusal>,
    },
    /// The lead gun's fuze is present on one side only, or its refusal or burst rings differ.
    Fuze {
        /// The client's refusal, `None` without a fuze or a refusal.
        client_refusal: Option<FuzeRefusal>,
        /// The server's refusal.
        server_refusal: Option<FuzeRefusal>,
        /// The client's burst-aim rings.
        client_burst_rings: Option<u32>,
        /// The server's burst-aim rings.
        server_burst_rings: Option<u32>,
    },
    /// A value differs by more than its tolerance, or is present on one side only.
    Value {
        /// The row.
        row: ComparedRow,
        /// The quantity.
        quantity: ComparedQuantity,
        /// The client's value.
        client: Option<f64>,
        /// The server's value.
        server: Option<f64>,
        /// The tolerance it exceeds.
        tolerance: f64,
    },
}

/// Compares the `client` solution with the `server` re-solve under the weapon convention of
/// `mils_per_circle`.
pub fn compare_solutions(
    client: &FireMissionSolution,
    server: &FireMissionSolution,
    mils_per_circle: u32,
) -> SolutionComparison {
    let mut found = Vec::new();
    for (field, client_value, server_value) in [
        (
            ProvenanceField::CatalogId,
            client.catalog_id.clone(),
            server.catalog_id.clone(),
        ),
        (
            ProvenanceField::CatalogVersion,
            client.catalog_version.to_string(),
            server.catalog_version.to_string(),
        ),
        (
            ProvenanceField::SolverRevision,
            client.solver_revision.clone(),
            server.solver_revision.clone(),
        ),
    ] {
        if client_value != server_value {
            found.push(SolutionMismatch::Provenance {
                field,
                client: client_value,
                server: server_value,
            });
        }
    }
    if client.guns.len() != server.guns.len() {
        found.push(SolutionMismatch::GunCount {
            client: client.guns.len(),
            server: server.guns.len(),
        });
    }
    let circle = f64::from(mils_per_circle);
    for (gun_index, (client_gun, server_gun)) in client.guns.iter().zip(&server.guns).enumerate() {
        if client_gun.mils_per_circle != server_gun.mils_per_circle
            || server_gun.mils_per_circle != mils_per_circle
        {
            found.push(SolutionMismatch::MilsPerCircle {
                gun_index,
                client: client_gun.mils_per_circle,
                server: server_gun.mils_per_circle,
            });
        }
        if client_gun.recommended_rings != server_gun.recommended_rings {
            found.push(SolutionMismatch::RecommendedCharge {
                gun_index,
                client: client_gun.recommended_rings,
                server: server_gun.recommended_rings,
            });
        }
        let rings = |charges: &[ChargeSolution]| charges.iter().map(|row| row.rings).collect();
        let (client_rings, server_rings): (Vec<u32>, Vec<u32>) =
            (rings(&client_gun.charges), rings(&server_gun.charges));
        if client_rings != server_rings {
            found.push(SolutionMismatch::ChargeRings {
                gun_index,
                client: client_rings,
                server: server_rings,
            });
            continue;
        }
        for (client_row, server_row) in client_gun.charges.iter().zip(&server_gun.charges) {
            let row = ComparedRow::GunCharge {
                gun_index,
                rings: server_row.rings,
            };
            compare_charge_row(&mut found, row, client_row, server_row, circle);
        }
    }
    compare_fuze(
        &mut found,
        client.fuze.as_ref(),
        server.fuze.as_ref(),
        circle,
    );
    SolutionComparison { mismatches: found }
}

/// Flags the refusal and the aim azimuth, elevation and time of flight of one charge row.
fn compare_charge_row(
    found: &mut Vec<SolutionMismatch>,
    row: ComparedRow,
    client: &ChargeSolution,
    server: &ChargeSolution,
    circle: f64,
) {
    if client.refusal != server.refusal {
        found.push(SolutionMismatch::ChargeRefusal {
            row,
            client: client.refusal,
            server: server.refusal,
        });
    }
    compare_aim(
        found,
        row,
        (client.aim_azimuth_mils, server.aim_azimuth_mils),
        (client.elevation_mils, server.elevation_mils),
        circle,
    );
    compare_value(
        found,
        row,
        ComparedQuantity::TimeOfFlightS,
        (client.time_of_flight_s, server.time_of_flight_s),
        TIME_TOLERANCE_S,
        None,
    );
}

/// Flags the fuze's presence, refusal, burst rings, fuze time and burst-point aim.
fn compare_fuze(
    found: &mut Vec<SolutionMismatch>,
    client: Option<&FireMissionFuze>,
    server: Option<&FireMissionFuze>,
    circle: f64,
) {
    let refusal = |fuze: Option<&FireMissionFuze>| fuze.and_then(|fuze| fuze.refusal);
    let burst_rings =
        |fuze: Option<&FireMissionFuze>| fuze.and_then(|fuze| fuze.burst_aim.map(|aim| aim.rings));
    if client.is_some() != server.is_some()
        || refusal(client) != refusal(server)
        || burst_rings(client) != burst_rings(server)
    {
        found.push(SolutionMismatch::Fuze {
            client_refusal: refusal(client),
            server_refusal: refusal(server),
            client_burst_rings: burst_rings(client),
            server_burst_rings: burst_rings(server),
        });
    }
    let (Some(client), Some(server)) = (client, server) else {
        return;
    };
    compare_value(
        found,
        ComparedRow::Fuze,
        ComparedQuantity::FuzeTimeS,
        (client.time_s, server.time_s),
        TIME_TOLERANCE_S,
        None,
    );
    let (client_aim, server_aim) = (client.burst_aim, server.burst_aim);
    compare_aim(
        found,
        ComparedRow::Fuze,
        (
            client_aim.map(|aim| aim.aim_azimuth_mils),
            server_aim.map(|aim| aim.aim_azimuth_mils),
        ),
        (
            client_aim.map(|aim| aim.elevation_mils),
            server_aim.map(|aim| aim.elevation_mils),
        ),
        circle,
    );
}

/// Flags an aim azimuth pair (circular over `circle` mils) and an elevation pair.
fn compare_aim(
    found: &mut Vec<SolutionMismatch>,
    row: ComparedRow,
    azimuth_mils: (Option<f64>, Option<f64>),
    elevation_mils: (Option<f64>, Option<f64>),
    circle: f64,
) {
    compare_value(
        found,
        row,
        ComparedQuantity::AimAzimuthMils,
        azimuth_mils,
        ANGLE_TOLERANCE_MILS,
        Some(circle),
    );
    compare_value(
        found,
        row,
        ComparedQuantity::ElevationMils,
        elevation_mils,
        ANGLE_TOLERANCE_MILS,
        None,
    );
}

/// Flags `(client, server)` unless both are absent or both are present within `tolerance`;
/// with a `circle` the difference is the shorter way round it.
fn compare_value(
    found: &mut Vec<SolutionMismatch>,
    row: ComparedRow,
    quantity: ComparedQuantity,
    (client, server): (Option<f64>, Option<f64>),
    tolerance: f64,
    circle: Option<f64>,
) {
    let agrees = match (client, server) {
        (None, None) => true,
        (Some(client_value), Some(server_value)) => {
            let difference = match circle {
                Some(circle) => libm::remainder(client_value - server_value, circle),
                None => client_value - server_value,
            };
            difference.abs() <= tolerance
        }
        _ => false,
    };
    if !agrees {
        found.push(SolutionMismatch::Value {
            row,
            quantity,
            client,
            server,
            tolerance,
        });
    }
}

#[cfg(test)]
#[path = "tests/fire_mission_comparison.rs"]
mod tests;
