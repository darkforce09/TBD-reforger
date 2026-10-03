//! The elevation of every native ballistic table row, fixed by the oracle's forward samples.
//!
//! **Role:** A game table row stores range and time of flight but not the elevation it was
//! computed at. The oracle samples the engine's forward lookup
//! (`BallisticTable.GetDistanceOfProjectileSource`) on an elevation lattice fine enough to hold
//! every row's elevation, so each row's elevation is fixed by one of two pieces of evidence:
//!
//! 1. **Forward sample:** exactly one sample's range and time of flight equal the row's within
//!    [`RANGE_TOLERANCE_M`] and [`TIME_TOLERANCE_S`] (samples the engine answered with its
//!    time-of-flight sentinel −1 never match).
//! 2. **Lattice end:** the engine answers both lattice ends with the sentinel instead of the row
//!    there, so the first row is the vertical shot at the lattice's first elevation when its
//!    range is 0 within [`RANGE_TOLERANCE_M`], and the last row sits at the lattice's last
//!    elevation when the sample there reports the row's range within [`RANGE_TOLERANCE_M`].
//!
//! A row neither rule fixes is an unmatched row and a hard error naming it.
//!
//! **Position:** Called by the trim once per native table at a catalog charge coefficient, with
//! that coefficient's forward samples; its result becomes each row's `elevation_mils_6400`.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** Every row is fixed; fixed elevations strictly decrease with the row index; and
//! every non-sentinel sample between two adjacent rows equals their linear interpolation within
//! tolerance, as the engine's lookup answers, or the table is refused.
use super::oracle_output::ForwardLattice;
use crate::error::{Result, refuse};

/// Range agreement between a row and a forward sample, in metres.
pub(crate) const RANGE_TOLERANCE_M: f64 = 0.01;
/// Time-of-flight agreement between a row and a forward sample, in seconds.
pub(crate) const TIME_TOLERANCE_S: f64 = 0.001;

/// One forward-angle sample: an elevation and the engine's range and time of flight there.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ForwardPoint {
    pub(crate) elevation_mils: f64,
    pub(crate) range_m: f64,
    pub(crate) time_of_flight_s: f64,
}

/// What fixed a row's elevation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ElevationEvidence {
    /// One forward sample equals the row.
    ForwardSample,
    /// The row sits at a lattice end the engine answers with the sentinel.
    LatticeEnd,
}

/// A row's elevation in 6400-mil units and the evidence that fixed it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RowElevation {
    pub(crate) elevation_mils: f64,
    pub(crate) evidence: ElevationEvidence,
}

/// A table row as `[range_m, column_1, time_of_flight_s]`.
type Row = [f64; 3];

fn agrees(range_m: f64, time_of_flight_s: f64, point: &ForwardPoint) -> bool {
    (range_m - point.range_m).abs() <= RANGE_TOLERANCE_M
        && (time_of_flight_s - point.time_of_flight_s).abs() <= TIME_TOLERANCE_S
}

/// Linear interpolation at `elevation` between `(upper elevation, row)` and `(lower, row)`.
fn interpolate(elevation: f64, upper: (f64, &Row), lower: (f64, &Row)) -> (f64, f64) {
    let fraction = (upper.0 - elevation) / (upper.0 - lower.0);
    (
        upper.1[0] + (lower.1[0] - upper.1[0]) * fraction,
        upper.1[2] + (lower.1[2] - upper.1[2]) * fraction,
    )
}

fn describe(index: usize, row: &Row) -> String {
    format!(
        "row {index} (range {} m, time of flight {} s)",
        row[0], row[2]
    )
}

/// The lattice-end elevation of row `index`, when the lattice-end rule fixes it.
fn lattice_end(
    index: usize,
    rows: &[Row],
    points: &[ForwardPoint],
    lattice: ForwardLattice,
) -> Option<f64> {
    let row = &rows[index];
    if index == 0 && row[0].abs() <= RANGE_TOLERANCE_M {
        return Some(lattice.first_mils);
    }
    let last_point = points
        .iter()
        .find(|point| point.elevation_mils == lattice.last_mils)?;
    (index > 0
        && index + 1 == rows.len()
        && last_point.time_of_flight_s < 0.0
        && (last_point.range_m - row[0]).abs() <= RANGE_TOLERANCE_M)
        .then_some(lattice.last_mils)
}

/// The elevation of every row of one table, from the forward samples at its coefficient.
pub(crate) fn assign_row_elevations(
    rows: &[Row],
    points: &[ForwardPoint],
    lattice: ForwardLattice,
) -> Result<Vec<RowElevation>> {
    if rows.is_empty() {
        refuse!("the table has no rows");
    }
    let observed: Vec<&ForwardPoint> = points
        .iter()
        .filter(|point| point.time_of_flight_s >= 0.0)
        .collect();
    let mut elevations = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let matches: Vec<&&ForwardPoint> = observed
            .iter()
            .filter(|point| agrees(row[0], row[2], point))
            .collect();
        let elevation = match matches.as_slice() {
            [point] => RowElevation {
                elevation_mils: point.elevation_mils,
                evidence: ElevationEvidence::ForwardSample,
            },
            [] => match lattice_end(index, rows, points, lattice) {
                Some(elevation_mils) => RowElevation {
                    elevation_mils,
                    evidence: ElevationEvidence::LatticeEnd,
                },
                None => refuse!(
                    "{} matches no forward sample within {RANGE_TOLERANCE_M} m and {TIME_TOLERANCE_S} s",
                    describe(index, row)
                ),
            },
            _ => refuse!(
                "{} matches {} forward samples",
                describe(index, row),
                matches.len()
            ),
        };
        elevations.push(elevation);
    }
    if let Some(pair) = (1..rows.len())
        .find(|&index| elevations[index - 1].elevation_mils <= elevations[index].elevation_mils)
    {
        refuse!(
            "row {} at {} mils is not above row {pair} at {} mils",
            pair - 1,
            elevations[pair - 1].elevation_mils,
            elevations[pair].elevation_mils
        );
    }
    check_interpolation(rows, &elevations, &observed)?;
    Ok(elevations)
}

/// Every observed sample between two adjacent rows equals their interpolation.
fn check_interpolation(
    rows: &[Row],
    elevations: &[RowElevation],
    observed: &[&ForwardPoint],
) -> Result<()> {
    for pair in 0..rows.len().saturating_sub(1) {
        let (upper, lower) = (
            elevations[pair].elevation_mils,
            elevations[pair + 1].elevation_mils,
        );
        for point in observed
            .iter()
            .filter(|point| point.elevation_mils <= upper && point.elevation_mils >= lower)
        {
            let (range_m, time_of_flight_s) = interpolate(
                point.elevation_mils,
                (upper, &rows[pair]),
                (lower, &rows[pair + 1]),
            );
            if !agrees(range_m, time_of_flight_s, point) {
                refuse!(
                    "the forward sample at {} mils (range {} m, time of flight {} s) is not the interpolation of rows {pair} and {} ({range_m} m, {time_of_flight_s} s)",
                    point.elevation_mils,
                    point.range_m,
                    point.time_of_flight_s,
                    pair + 1
                );
            }
        }
    }
    Ok(())
}
