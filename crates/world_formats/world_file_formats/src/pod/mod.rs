//! The world object row a `TBDC` chunk carries.
//!
//! **Role:** declares [`instance::ObjectInstancePod`], the 32-byte row that is both the on-disk
//! layout and the GPU instance layout, and its zero-copy byte casts.
//! **Position:** written by the developer tools' world export inside
//! [`crate::containers::tbdc`] chunks; read by the map engine's chunk loader.
//! **Signals & state:** none; a `Pod` data type and pure functions.
//! **Invariants:** the row is exactly [`instance::POD_BYTES`] long and little-endian; its name is
//! recorded in the terrain manifest as [`instance::POD_NAME`].

pub mod instance;
