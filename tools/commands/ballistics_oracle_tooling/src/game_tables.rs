//! The game's own ballistic and wind tables of one shell, read from the gameplay export.
//!
//! **Role:** Reads a shell's `BallisticTableArray` configuration (its indirect-fire
//! `BallisticTable` entries, each one muzzle speed coefficient with rows of range, the
//! uninterpreted second column and time of flight) and its `SCR_ProjectileWindTable`
//! configuration (per coefficient and wind speed, the firing-solution rows and the value rows the
//! game pairs with them).
//!
//! **Position:** Called by the trim for every catalog shell; the rows it returns get their
//! elevations from [`super::row_elevations`] and become the bundle's `native_tables` and
//! `wind_tables`.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** Rows keep the game's order, so a row's index is its `lattice_index`. Every
//! number passes through [`engine_number`]. A table row that is not three numbers, and a wind
//! table whose firing-solution and value lists differ in length, are errors. The direct-fire
//! table list is not read: mortars fire indirect only.
use super::engine_numbers::engine_number;
use super::gameplay_export::ExportedResource;
use crate::error::{Result, ResultExt, refuse};
use serde_json::Value;

/// One game table at one muzzle speed coefficient.
pub(crate) struct GameTable {
    pub(crate) init_speed_coef: f64,
    /// `[range_m, column_1, time_of_flight_s]` per row, in the game's order.
    pub(crate) rows: Vec<[f64; 3]>,
}

/// One wind table at one coefficient and wind speed.
pub(crate) struct GameWindTable {
    pub(crate) init_speed_coef: f64,
    pub(crate) wind_speed_m_s: f64,
    /// `[elevation_rad, range_m, apex_m]` per row.
    pub(crate) firing_solutions: Vec<[f64; 3]>,
    /// The game's value list paired with each firing-solution row.
    pub(crate) values: Vec<Vec<f64>>,
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_f64)
        .map(engine_number)
        .collect()
}

fn triples(resource: &ExportedResource, value: &Value, what: &str) -> Result<Vec<[f64; 3]>> {
    value
        .as_array()
        .with_context(|| format!("{} {what} is not a list", resource.resource_name))?
        .iter()
        .map(|row| match numbers(row).as_slice() {
            [first, second, third] => Ok([*first, *second, *third]),
            _ => refuse!(
                "{} {what} row {row} is not three numbers",
                resource.resource_name
            ),
        })
        .collect()
}

fn coefficient(resource: &ExportedResource, node: &Value, name: &str) -> Result<f64> {
    resource
        .property(node, name)?
        .as_f64()
        .map(engine_number)
        .with_context(|| format!("{} {name} is not a number", resource.resource_name))
}

/// Every indirect-fire table of a `BallisticTableArray` configuration.
pub(crate) fn native_tables(resource: &ExportedResource) -> Result<Vec<GameTable>> {
    let root = resource.single_node("BallisticTableArray")?;
    resource
        .referenced_nodes(root, "Indirect fire Table data")?
        .into_iter()
        .map(|table| {
            Ok(GameTable {
                init_speed_coef: coefficient(resource, table, "InitSpeedCoefficient")?,
                rows: triples(
                    resource,
                    resource.property(table, "Table data")?,
                    "Table data",
                )?,
            })
        })
        .collect()
}

/// Every wind table of an `SCR_ProjectileWindTable` configuration.
pub(crate) fn wind_tables(resource: &ExportedResource) -> Result<Vec<GameWindTable>> {
    let root = resource.single_node("SCR_ProjectileWindTable")?;
    resource
        .referenced_nodes(root, "m_aData")?
        .into_iter()
        .map(|data| {
            let firing_solutions = triples(
                resource,
                resource.property(data, "m_aFiringSolution")?,
                "m_aFiringSolution",
            )?;
            let values: Vec<Vec<f64>> = resource
                .property(data, "m_aValues")?
                .as_array()
                .into_iter()
                .flatten()
                .map(numbers)
                .collect();
            if values.len() != firing_solutions.len() {
                refuse!(
                    "{} wind data holds {} firing solutions but {} value rows",
                    resource.resource_name,
                    firing_solutions.len(),
                    values.len()
                );
            }
            Ok(GameWindTable {
                init_speed_coef: coefficient(resource, data, "m_fInitSpeedCoef")?,
                wind_speed_m_s: coefficient(resource, data, "m_fWindSpeed")?,
                firing_solutions,
                values,
            })
        })
        .collect()
}
