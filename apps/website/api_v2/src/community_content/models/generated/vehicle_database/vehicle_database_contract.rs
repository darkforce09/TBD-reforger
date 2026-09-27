// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/vehicle-database.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::Vehicle;

///The vehicle database, the identification table of the doctrine pages: its rows, the list, and the administrator's writes. The root is one row.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct VehicleDatabaseContract(pub Vehicle);
impl ::std::ops::Deref for VehicleDatabaseContract {
    type Target = Vehicle;
    fn deref(&self) -> &Vehicle {
        &self.0
    }
}
impl ::std::convert::From<Vehicle> for VehicleDatabaseContract {
    fn from(value: Vehicle) -> Self {
        Self(value)
    }
}
