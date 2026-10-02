//! The on-disk formats of a terrain's served map data.
//!
//! **Role:** defines the byte layout of every file a terrain's map data ships in, with the one
//! writer and the one validating reader of each: the rkyv archives ([`archives`]), the four
//! fixed-header binary containers ([`containers`]), the vegetation density grid ([`density`]) and
//! the world object row ([`pod`]), and the identifier types their records hold ([`ids`]).
//! **Position:** world formats category, tier 1, depending on `rkyv`, `bytemuck`, `thiserror` and
//! the `newtype_ids` macros. The developer tools' export and raster pipelines write these files;
//! the map engine's loaders read them, natively and on wasm32.
//! **Signals & state:** none; plain data types and pure functions over byte slices.
//! **Invariants:** the byte format is frozen: rkyv stays `little_endian` with `bytecheck`
//! validation, every container header is 32 bytes, and every committed map asset and fixture keeps
//! loading. A malformed buffer is an error value, never a panic.

pub mod archives;
pub mod containers;
pub mod density;
mod error;
pub mod ids;
pub mod pod;
pub mod prelude;

pub use error::{Error, Result};
