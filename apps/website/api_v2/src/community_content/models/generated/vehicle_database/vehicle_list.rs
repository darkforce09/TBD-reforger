// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/vehicle-database.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::Vehicle;

///GET /api/v1/vehicle-database (any signed-in member): every row that is not deleted, ordered by name, then id.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct VehicleList {
    pub data: ::std::vec::Vec<Vehicle>,
}
