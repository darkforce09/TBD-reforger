//! Game ballistics catalogs: the judgement of an uploaded catalog pair and the immutable store.
//!
//! **Role:** the logic behind the catalog routes: [`upload_validation`] decodes an uploaded
//! catalog and calibration bundle and judges them with the map engine's calibration;
//! [`catalog_store`] checks duplicates, stores an accepted version with its audit line, and reads
//! stored versions back.
//!
//! **Position:** operations services; used by
//! [`crate::operations::handlers::ballistics_catalogs`] and by the fire-mission save, which pins
//! a stored version through [`catalog_store::load_catalog`].
//!
//! **Signals & state:** none; the `ballistics_catalogs` table is the only state.
//!
//! **Invariants:** a version is stored only after its calibration lists no failure, and a stored
//! version never changes.

pub mod catalog_store;
pub mod upload_validation;
