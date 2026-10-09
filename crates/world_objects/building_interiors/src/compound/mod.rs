//! A building compound: its collision shell plus every entity placed in it.
//!
//! **Role:** declares the compound assembly ([`assembly`]), the instances file model and its live
//! instances ([`instances`]) and the door records, states and lookups ([`doors`]).
//! **Position:** under the crate root; the map engine's interior line of sight, visibility wash
//! and world line of sight, the Mission Creator's building benches and the developer tools'
//! blueprint pipeline read it.
//! **Signals & state:** a compound's door states, set by its owner.
//! **Invariants:** a compound assembles atomically: either every record's BLAS is present or the
//! assembly is refused with the missing paths.

pub mod assembly;
pub mod doors;
pub mod instances;
