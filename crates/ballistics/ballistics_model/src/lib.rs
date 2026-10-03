//! The vocabulary of game ballistics: the catalog, the shell flight model, wind and angles.
//!
//! **Role:** the typed form of a ballistics catalog (weapons, shells, charges and time fuzes) with
//! its decode and typed lookups ([`catalog`]), the engine's point-mass shell flight with its 1/30 s
//! step and quadratic drag ([`flight_model`]), the constant surface wind ([`wind`]), degrees,
//! radians and weapon mils ([`angular_units`]) and the catalog identifiers ([`ids`]).
//! **Position:** ballistics tier 1, over `newtype_ids`. The firing solver, the fire-mission
//! planner, the calibration and the agreement cases build on it; the API, the single-page app and
//! the developer tools decode catalogs through it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** catalog field names and shapes match
//! `contracts/definitions/ballistics-catalog.schema.json`; transcendentals come from `libm`, so
//! a flight has the same bits on native and wasm32 builds.

pub mod angular_units;
pub mod catalog;
mod error;
pub mod flight_model;
pub mod ids;
pub mod prelude;
pub mod wind;

/// The crate's error and result.
pub use error::{Error, Result};
/// The catalog, export generation, weapon and shell identifiers.
pub use ids::{CatalogId, ExportGenerationId, ShellId, WeaponId};
