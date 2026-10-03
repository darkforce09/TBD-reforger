//! A building's blueprint: its levels and what stands on them, and what a sight line meets.
//!
//! **Role:** declares the blueprint model ([`structure`], [`footprint`]), its rebuild from the
//! archive ([`archive`]), the sight-line attribution ([`sight_line`]) and the per-level aperture,
//! cover and stairwell annotations it gathers ([`level_annotations`]).
//! **Position:** under the crate root; read by the compound and section modules, the map engine's
//! interior line of sight, the Mission Creator's building viewer and the developer tools'
//! blueprint parity report.
//! **Signals & state:** none.
//! **Invariants:** the blueprint never decides a sight line's verdict; the occlusion mesh does.

pub mod archive;
pub mod footprint;
pub mod level_annotations;
pub mod sight_line;
pub mod structure;

#[cfg(test)]
#[path = "tests/sight_line_tests.rs"]
mod tests;
