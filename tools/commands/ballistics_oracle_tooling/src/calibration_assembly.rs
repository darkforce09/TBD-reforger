//! The calibration cases of one catalog shell: its native tables, wind tables and oracle samples.
//!
//! **Role:** For each charge of a shell, finds the game table at the charge's coefficient, fixes
//! its row elevations from the forward samples at that coefficient, keeps the wind tables at
//! charge coefficients, and converts the oracle's forward-angle, simulation and
//! altitude-difference samples of the shell into bundle samples.
//!
//! **Position:** Called by the trim once per shell of the extracted catalog; the trim concatenates
//! the results into the calibration bundle.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** Only charge coefficients enter the bundle: tables and forward samples at other
//! coefficients are trimmed, and a simulation or altitude sample at a coefficient that is not a
//! charge is an error. Every charge has exactly one game table. Forward samples the engine
//! answered with the time-of-flight sentinel −1 are left out: they are lattice-end answers, not
//! ballistic results. Every native row of a kept table enters the bundle, its elevation fixed by
//! [`assign_row_elevations`] or the trim refused. Each sample's `inputs` gains the charge's `rings` in front of the engine's
//! arguments.
use super::catalog_extraction::ExtractedShell;
use super::contract_documents::{
    NativeTable, NativeTableRow, OracleSample, WindTable, WindTableRow,
};
use super::game_tables::{GameTable, GameWindTable};
use super::oracle_output::{ForwardLattice, OracleOutput};
use super::row_elevations::{ElevationEvidence, ForwardPoint, assign_row_elevations};
use crate::error::{Result, ResultExt, refuse};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The calibration cases of one shell.
#[derive(Default)]
pub(crate) struct ShellCalibration {
    pub(crate) native_tables: Vec<NativeTable>,
    pub(crate) wind_tables: Vec<WindTable>,
    pub(crate) samples: Vec<OracleSample>,
    /// How many rows each kind of evidence fixed.
    pub(crate) evidence_counts: BTreeMap<ElevationEvidence, usize>,
}

fn number(value: &Value, what: &str) -> Result<f64> {
    value
        .as_f64()
        .with_context(|| format!("oracle sample {what} is not a number"))
}

fn sample(
    kind: &str,
    shell_id: &str,
    rings: u32,
    init_speed_coef: f64,
    raw: &Value,
) -> Result<OracleSample> {
    let mut inputs = Map::new();
    inputs.insert("rings".to_owned(), Value::from(rings));
    for (key, value) in raw["inputs"]
        .as_object()
        .context("oracle sample has no inputs")?
    {
        inputs.insert(key.clone(), value.clone());
    }
    Ok(OracleSample {
        kind: kind.to_owned(),
        shell_id: shell_id.to_owned(),
        init_speed_coef,
        inputs,
        outputs: raw["outputs"]
            .as_object()
            .context("oracle sample has no outputs")?
            .clone(),
    })
}

fn native_table(
    shell_id: &str,
    table: &GameTable,
    forward: &[&Value],
    lattice: ForwardLattice,
    calibration: &mut ShellCalibration,
) -> Result<NativeTable> {
    let points = forward
        .iter()
        .map(|raw| {
            Ok(ForwardPoint {
                elevation_mils: number(
                    &raw["inputs"]["elevation_mils_6400"],
                    "elevation_mils_6400",
                )?,
                range_m: number(&raw["outputs"]["range_m"], "range_m")?,
                time_of_flight_s: number(&raw["outputs"]["time_of_flight_s"], "time_of_flight_s")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let elevations = assign_row_elevations(&table.rows, &points, lattice).with_context(|| {
        format!(
            "shell {shell_id} native table at coefficient {}: unmatched row",
            table.init_speed_coef
        )
    })?;
    let mut rows = Vec::with_capacity(table.rows.len());
    for (lattice_index, (row, elevation)) in table.rows.iter().zip(elevations).enumerate() {
        *calibration
            .evidence_counts
            .entry(elevation.evidence)
            .or_default() += 1;
        rows.push(NativeTableRow {
            lattice_index,
            elevation_mils_6400: elevation.elevation_mils,
            range_m: row[0],
            column_1: row[1],
            time_of_flight_s: row[2],
        });
    }
    Ok(NativeTable {
        shell_id: shell_id.to_owned(),
        init_speed_coef: table.init_speed_coef,
        rows,
    })
}

fn wind_table(shell_id: &str, table: &GameWindTable) -> WindTable {
    WindTable {
        shell_id: shell_id.to_owned(),
        init_speed_coef: table.init_speed_coef,
        wind_speed_m_s: table.wind_speed_m_s,
        rows: table
            .firing_solutions
            .iter()
            .zip(&table.values)
            .map(|(solution, values)| WindTableRow {
                elevation_rad: solution[0],
                range_m: solution[1],
                apex_m: solution[2],
                values: values.clone(),
            })
            .collect(),
    }
}

/// Every calibration case of `shell` from its game tables and the oracle output.
pub(crate) fn shell_calibration(
    shell: &ExtractedShell,
    tables: &[GameTable],
    wind_tables: &[GameWindTable],
    oracle: &OracleOutput,
) -> Result<ShellCalibration> {
    let shell_id = shell.shell.shell_id.as_str();
    let guid = shell.shell.prefab_guid.as_str();
    let mut calibration = ShellCalibration::default();
    let forward = oracle.forward_samples(guid)?;
    let charge_rings = |coefficient: f64| {
        shell
            .shell
            .charges
            .iter()
            .find(|charge| charge.init_speed_coef == coefficient)
            .map(|charge| charge.rings)
    };
    for charge in &shell.shell.charges {
        let coefficient = charge.init_speed_coef;
        let table = match tables
            .iter()
            .filter(|table| table.init_speed_coef == coefficient)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [table] => *table,
            found => refuse!(
                "shell {shell_id} has {} native tables at charge coefficient {coefficient}, expected one",
                found.len()
            ),
        };
        let at_coefficient: Vec<&Value> = forward
            .iter()
            .filter(|raw| raw["init_speed_coef"].as_f64() == Some(coefficient))
            .collect();
        let native = native_table(
            shell_id,
            table,
            &at_coefficient,
            oracle.lattice,
            &mut calibration,
        )?;
        calibration.native_tables.push(native);
        for table in wind_tables
            .iter()
            .filter(|table| table.init_speed_coef == coefficient)
        {
            calibration.wind_tables.push(wind_table(shell_id, table));
        }
        for raw in at_coefficient {
            if raw["outputs"]["time_of_flight_s"]
                .as_f64()
                .is_some_and(|time| time >= 0.0)
            {
                calibration.samples.push(sample(
                    "forward_angle",
                    shell_id,
                    charge.rings,
                    coefficient,
                    raw,
                )?);
            }
        }
    }
    for (kind, samples) in [
        ("simulation", oracle.simulation_samples(guid)?),
        ("altitude_difference", oracle.altitude_samples(guid)?),
    ] {
        for raw in samples {
            let coefficient = number(&raw["init_speed_coef"], "init_speed_coef")?;
            let Some(rings) = charge_rings(coefficient) else {
                refuse!(
                    "shell {shell_id} {kind} sample at coefficient {coefficient} is not at a charge coefficient"
                );
            };
            calibration
                .samples
                .push(sample(kind, shell_id, rings, coefficient, raw)?);
        }
    }
    Ok(calibration)
}
