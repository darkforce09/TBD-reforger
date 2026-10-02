//! The vegetation density grid format.
//!
//! **Role:** declares the `TBDD` codec ([`tbdd`]): one export chunk's tree and rock density
//! channels on a fixed cell grid.
//! **Position:** the developer tools' world export writes the tiles under
//! `objects/density/`; the map engine's vegetation layer decodes them.
//! **Signals & state:** none; pure functions over byte slices.
//! **Invariants:** a 16-byte little-endian header, then `u16` channels; a malformed buffer is a
//! [`tbdd::TbddError`], never a panic.

pub mod tbdd;
